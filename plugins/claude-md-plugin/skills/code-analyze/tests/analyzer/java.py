"""Java code analyzer using regex patterns."""

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


class JavaAnalyzer(Analyzer):
    """Analyzer for Java files."""

    @property
    def language(self) -> str:
        return "java"

    @property
    def file_extensions(self) -> list[str]:
        return [".java"]

    # Regex patterns for Java analysis
    PATTERNS = {
        # public ReturnType methodName(params) throws ...
        "public_method": re.compile(
            r"^\s*public\s+(?:static\s+)?(?:<[^>]+>\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)\s*(?:throws\s+([^{]+))?\s*\{",
            re.MULTILINE,
        ),
        # private ReturnType methodName(params)
        "private_method": re.compile(
            r"^\s*private\s+(?:static\s+)?(?:<[^>]+>\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)",
            re.MULTILINE,
        ),
        # public class ClassName extends/implements
        "public_class": re.compile(
            r"^public\s+(?:final\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?\s*\{",
            re.MULTILINE,
        ),
        # public enum EnumName
        "public_enum": re.compile(
            r"^public\s+enum\s+(\w+)\s*\{",
            re.MULTILINE,
        ),
        # public interface InterfaceName
        "public_interface": re.compile(
            r"^public\s+interface\s+(\w+)\s*\{",
            re.MULTILINE,
        ),
        # import package.Class
        "import": re.compile(
            r"^import\s+([\w.]+);",
            re.MULTILINE,
        ),
        # throws Exception
        "throws": re.compile(
            r"throws\s+([\w,\s]+)",
            re.MULTILINE,
        ),
        # throw new Exception
        "throw_new": re.compile(
            r"throw\s+new\s+(\w+Exception)",
            re.MULTILINE,
        ),
    }

    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a Java file."""
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
        """Extract all exports from Java content."""
        functions = []
        types = []
        classes = []

        # Extract public methods
        for match in self.PATTERNS["public_method"].finditer(content):
            return_type = match.group(1)
            name = match.group(2)
            params = match.group(3).strip()
            throws = match.group(4).strip() if match.group(4) else ""

            # Skip constructors (name matches class name pattern)
            if name[0].isupper():
                continue

            signature = f"{return_type} {name}({self._simplify_params(params)})"
            if throws:
                signature += f" throws {throws}"

            functions.append(ExportedFunction(
                name=name,
                signature=signature,
                description=self._extract_javadoc(content, match.start()),
            ))

        # Get private method names for exclusion checks
        private_methods = set()
        for match in self.PATTERNS["private_method"].finditer(content):
            private_methods.add(match.group(2))

        # Extract public classes
        for match in self.PATTERNS["public_class"].finditer(content):
            name = match.group(1)
            extends = match.group(2) or ""

            signature = f"public class {name}"
            if extends:
                signature += f" extends {extends}"

            # Determine if it's an exception class
            if extends and "Exception" in extends:
                types.append(ExportedType(
                    name=name,
                    kind="exception",
                    definition=signature,
                    description=self._extract_javadoc(content, match.start()),
                ))
            else:
                classes.append(ExportedClass(
                    name=name,
                    signature=signature,
                    description=self._extract_javadoc(content, match.start()),
                ))

        # Extract public enums
        for match in self.PATTERNS["public_enum"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="enum",
                definition=f"public enum {name}",
                description=self._extract_javadoc(content, match.start()),
            ))

        # Extract public interfaces
        for match in self.PATTERNS["public_interface"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="interface",
                definition=f"public interface {name}",
                description=self._extract_javadoc(content, match.start()),
            ))

        return Exports(functions=functions, types=types, classes=classes)

    def _extract_dependencies(self, content: str) -> Dependencies:
        """Extract dependencies from import statements."""
        external = []
        internal = []

        java_pkgs = {"java", "javax", "sun"}

        for match in self.PATTERNS["import"].finditer(content):
            package = match.group(1)
            top_level = package.split(".")[0]

            if top_level in java_pkgs:
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
        for match in self.PATTERNS["throw_new"].finditer(content):
            thrown_exceptions.add(match.group(1))

        # Look for declared throws
        for match in self.PATTERNS["throws"].finditer(content):
            exceptions_str = match.group(1)
            for exc in exceptions_str.split(","):
                exc = exc.strip()
                if exc:
                    thrown_exceptions.add(exc)

        # Add success behavior
        if "return" in content:
            behaviors.append(Behavior(
                input="Valid JWT token",
                output="TokenClaims object",
                category="success",
            ))

        # Add error behaviors
        if "TokenExpiredException" in thrown_exceptions or "Expired" in content:
            behaviors.append(Behavior(
                input="Expired token",
                output="TokenExpiredException",
                category="error",
            ))

        if "InvalidTokenException" in thrown_exceptions or "Invalid" in content:
            behaviors.append(Behavior(
                input="Invalid token",
                output="InvalidTokenException",
                category="error",
            ))

        return behaviors

    def _extract_javadoc(self, content: str, position: int) -> str:
        """Extract Javadoc comment before a position."""
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
                # Take just type and name
                words = part.split()
                if len(words) >= 2:
                    parts.append(f"{words[-2]} {words[-1]}")
        return ", ".join(parts[:3]) + ("..." if len(parts) > 3 else "")
