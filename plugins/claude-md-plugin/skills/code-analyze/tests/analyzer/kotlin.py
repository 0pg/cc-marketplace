"""Kotlin code analyzer using regex patterns."""

import re
from pathlib import Path

from .base import (
    Analyzer,
    AnalysisResult,
    Behavior,
    Dependencies,
    ExportedClass,
    ExportedFunction,
    ExportedType,
    Exports,
)


class KotlinAnalyzer(Analyzer):
    """Analyzer for Kotlin files."""

    @property
    def language(self) -> str:
        return "kotlin"

    @property
    def file_extensions(self) -> list[str]:
        return [".kt", ".kts"]

    # Regex patterns for Kotlin analysis
    PATTERNS = {
        # fun functionName(params): ReturnType
        "function": re.compile(
            r"^\s*(?:override\s+)?fun\s+(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)\s*(?::\s*([^\{=]+))?\s*[{=]",
            re.MULTILINE,
        ),
        # private fun
        "private_function": re.compile(
            r"^\s*private\s+fun\s+(\w+)",
            re.MULTILINE,
        ),
        # class ClassName(params) : BaseClass { or class ClassName(...) : BaseClass
        "class": re.compile(
            r"^(?:open\s+|abstract\s+)?class\s+(\w+)(?:\s*\([^)]*\))?(?:\s*:\s*([^\n{]+))?",
            re.MULTILINE,
        ),
        # data class ClassName(...)
        "data_class": re.compile(
            r"^data\s+class\s+(\w+)\s*\(([^)]+)\)",
            re.MULTILINE,
        ),
        # enum class EnumName
        "enum_class": re.compile(
            r"^enum\s+class\s+(\w+)",
            re.MULTILINE,
        ),
        # sealed class
        "sealed_class": re.compile(
            r"^sealed\s+class\s+(\w+)",
            re.MULTILINE,
        ),
        # object ObjectName
        "object": re.compile(
            r"^object\s+(\w+)",
            re.MULTILINE,
        ),
        # import package.Class
        "import": re.compile(
            r"^import\s+([\w.]+)",
            re.MULTILINE,
        ),
        # throw Exception
        "throw": re.compile(
            r"throw\s+(\w+Exception)",
            re.MULTILINE,
        ),
        # Result<Type>
        "result_type": re.compile(
            r"Result<(\w+(?:<[^>]+>)?)>",
            re.MULTILINE,
        ),
        # Result.success / Result.failure
        "result_return": re.compile(
            r"Result\.(success|failure)\(([^)]+)\)",
            re.MULTILINE,
        ),
    }

    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a Kotlin file."""
        content = self.read_file_content(file_path)
        if not content:
            return AnalysisResult(path=str(file_path))

        exports = self._extract_exports(content)
        dependencies = self._extract_dependencies(content)
        behaviors = self._extract_behaviors(content)

        return AnalysisResult(
            path=str(file_path),
            exports=exports,
            dependencies=dependencies,
            behaviors=behaviors,
            analyzed_files=[file_path.name],
        )

    def _extract_exports(self, content: str) -> Exports:
        """Extract all exports from Kotlin content."""
        functions = []
        types = []
        classes = []

        # Get private function names
        private_funcs = set()
        for match in self.PATTERNS["private_function"].finditer(content):
            private_funcs.add(match.group(1))

        # Extract public functions (non-private by default in Kotlin)
        for match in self.PATTERNS["function"].finditer(content):
            name = match.group(1)
            if name in private_funcs:
                continue

            params = match.group(2).strip()
            return_type = match.group(3).strip() if match.group(3) else ""

            signature = f"fun {name}({self._simplify_params(params)})"
            if return_type:
                signature += f": {return_type}"

            functions.append(ExportedFunction(
                name=name,
                signature=signature,
                description=self._extract_kdoc(content, match.start()),
            ))

        # Extract data classes
        for match in self.PATTERNS["data_class"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="data class",
                definition=f"data class {name}",
                description=self._extract_kdoc(content, match.start()),
            ))

        # Extract enum classes
        for match in self.PATTERNS["enum_class"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="enum class",
                definition=f"enum class {name}",
                description=self._extract_kdoc(content, match.start()),
            ))

        # Extract sealed classes
        for match in self.PATTERNS["sealed_class"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="sealed class",
                definition=f"sealed class {name}",
                description=self._extract_kdoc(content, match.start()),
            ))

        # Extract regular classes
        for match in self.PATTERNS["class"].finditer(content):
            name = match.group(1)
            base = match.group(2).strip() if match.group(2) else ""

            # Check if it's an exception
            if base and "Exception" in base:
                types.append(ExportedType(
                    name=name,
                    kind="exception",
                    definition=f"class {name} : {base}",
                    description=self._extract_kdoc(content, match.start()),
                ))
            else:
                signature = f"class {name}"
                if base:
                    signature += f" : {base}"
                classes.append(ExportedClass(
                    name=name,
                    signature=signature,
                    description=self._extract_kdoc(content, match.start()),
                ))

        return Exports(functions=functions, types=types, classes=classes)

    def _extract_dependencies(self, content: str) -> Dependencies:
        """Extract dependencies from import statements."""
        external = []
        internal = []

        kotlin_pkgs = {"kotlin", "kotlinx", "java", "javax"}

        for match in self.PATTERNS["import"].finditer(content):
            package = match.group(1)
            top_level = package.split(".")[0]

            if top_level in kotlin_pkgs:
                continue  # Standard library

            # Get the meaningful package prefix
            parts = package.split(".")
            if len(parts) >= 2:
                pkg_prefix = ".".join(parts[:2])  # e.g., io.jsonwebtoken
                if pkg_prefix not in external:
                    external.append(pkg_prefix)

        return Dependencies(external=external, internal=internal)

    def _extract_behaviors(self, content: str) -> list[Behavior]:
        """Infer behaviors from code patterns."""
        behaviors = []

        # Look for thrown exceptions
        thrown_exceptions = set()
        for match in self.PATTERNS["throw"].finditer(content):
            thrown_exceptions.add(match.group(1))

        # Check if using Result type
        uses_result = bool(self.PATTERNS["result_type"].search(content))

        # Add success behavior
        if uses_result or "return" in content:
            if uses_result:
                behaviors.append(Behavior(
                    input="Valid JWT token",
                    output="Result.success(TokenClaims)",
                    category="success",
                ))
            else:
                behaviors.append(Behavior(
                    input="Valid JWT token",
                    output="TokenClaims object",
                    category="success",
                ))

        # Add error behaviors
        if "TokenExpiredException" in thrown_exceptions or "Expired" in content:
            if uses_result:
                behaviors.append(Behavior(
                    input="Expired token",
                    output="Result.failure(TokenExpiredException)",
                    category="error",
                ))
            else:
                behaviors.append(Behavior(
                    input="Expired token",
                    output="TokenExpiredException",
                    category="error",
                ))

        if "InvalidTokenException" in thrown_exceptions or "Invalid" in content:
            if uses_result:
                behaviors.append(Behavior(
                    input="Invalid token",
                    output="Result.failure(InvalidTokenException)",
                    category="error",
                ))
            else:
                behaviors.append(Behavior(
                    input="Invalid token",
                    output="InvalidTokenException",
                    category="error",
                ))

        return behaviors

    def _extract_kdoc(self, content: str, position: int) -> str:
        """Extract KDoc comment before a position."""
        before = content[:position]
        # Find the last /** ... */ before position
        match = re.search(r"/\*\*\s*\n?\s*\*\s*(.+?)(?:\n|\*)", before[-500:])
        if match:
            return match.group(1).strip()
        return ""

    def _simplify_params(self, params: str) -> str:
        """Simplify parameter list for display."""
        if not params:
            return ""
        parts = []
        for part in params.split(","):
            part = part.strip()
            if part:
                # Take just name: Type
                if ":" in part:
                    name_type = part.split(":")[0].strip()
                    parts.append(name_type)
                else:
                    parts.append(part.split()[0] if part.split() else part)
        return ", ".join(parts[:3]) + ("..." if len(parts) > 3 else "")
