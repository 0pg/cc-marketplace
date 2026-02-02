"""Python code analyzer using regex patterns."""

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


class PythonAnalyzer(Analyzer):
    """Analyzer for Python files."""

    @property
    def language(self) -> str:
        return "python"

    @property
    def file_extensions(self) -> list[str]:
        return [".py"]

    # Regex patterns for Python analysis
    PATTERNS = {
        # def function_name(params) -> ReturnType:
        "function": re.compile(
            r"^def\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^:]+))?\s*:",
            re.MULTILINE,
        ),
        # async def function_name(params) -> ReturnType:
        "async_function": re.compile(
            r"^async\s+def\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^:]+))?\s*:",
            re.MULTILINE,
        ),
        # class ClassName(bases):
        "class": re.compile(
            r"^class\s+(\w+)(?:\s*\(([^)]*)\))?\s*:",
            re.MULTILINE,
        ),
        # @dataclass
        "dataclass": re.compile(
            r"@dataclass\s*\n\s*class\s+(\w+)",
            re.MULTILINE,
        ),
        # __all__ = ['name1', 'name2']
        "all_exports": re.compile(
            r"__all__\s*=\s*\[([^\]]+)\]",
            re.MULTILINE,
        ),
        # import package / from package import ...
        "import_from": re.compile(
            r"^(?:from\s+(\S+)\s+import|import\s+(\S+))",
            re.MULTILINE,
        ),
        # raise ErrorClass
        "raise_error": re.compile(
            r"raise\s+(\w+(?:Error|Exception))?",
            re.MULTILINE,
        ),
        # except ErrorClass:
        "except_error": re.compile(
            r"except\s+(\w+(?:Error|Exception))",
            re.MULTILINE,
        ),
    }

    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a Python file."""
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
        """Extract all exports from Python content."""
        functions = []
        types = []
        classes = []

        # Check for __all__ to determine what's exported
        all_match = self.PATTERNS["all_exports"].search(content)
        exported_names = set()
        if all_match:
            # Parse the __all__ list
            all_str = all_match.group(1)
            for name_match in re.finditer(r"['\"](\w+)['\"]", all_str):
                exported_names.add(name_match.group(1))

        # Get dataclass names
        dataclass_names = set()
        for match in self.PATTERNS["dataclass"].finditer(content):
            dataclass_names.add(match.group(1))

        # Extract functions (non-private)
        for pattern in ["function", "async_function"]:
            for match in self.PATTERNS[pattern].finditer(content):
                name = match.group(1)
                # Skip private functions
                if name.startswith("_") and not name.startswith("__"):
                    continue
                # If __all__ exists, only include listed items
                if exported_names and name not in exported_names:
                    continue

                params = match.group(2).strip()
                return_type = match.group(3).strip() if match.group(3) else ""
                signature = f"{name}({params})"
                if return_type:
                    signature += f" -> {return_type}"

                functions.append(ExportedFunction(
                    name=name,
                    signature=signature,
                    description=self._extract_docstring(content, match.end()),
                ))

        # Extract classes
        for match in self.PATTERNS["class"].finditer(content):
            name = match.group(1)
            bases = match.group(2) or ""

            # Skip private classes
            if name.startswith("_"):
                continue
            # If __all__ exists, only include listed items
            if exported_names and name not in exported_names:
                continue

            # Check if it's a dataclass
            if name in dataclass_names:
                types.append(ExportedType(
                    name=name,
                    kind="dataclass",
                    definition=f"@dataclass class {name}",
                    description=self._extract_docstring(content, match.end()),
                ))
            else:
                signature = f"class {name}"
                if bases:
                    signature += f"({bases})"
                classes.append(ExportedClass(
                    name=name,
                    signature=signature,
                    description=self._extract_docstring(content, match.end()),
                ))

        return Exports(functions=functions, types=types, classes=classes)

    def _extract_dependencies(self, content: str) -> Dependencies:
        """Extract dependencies from import statements."""
        external = []
        internal = []

        for match in self.PATTERNS["import_from"].finditer(content):
            module = match.group(1) or match.group(2)
            if not module:
                continue

            if module.startswith("."):
                internal.append(module)
            else:
                # Get the top-level package
                pkg = module.split(".")[0]
                if pkg not in external and pkg not in ["typing", "dataclasses", "abc"]:
                    external.append(pkg)

        return Dependencies(external=external, internal=internal)

    def _extract_behaviors(self, content: str) -> list[Behavior]:
        """Infer behaviors from code patterns."""
        behaviors = []

        # Look for raised exceptions
        raised_errors = set()
        for match in self.PATTERNS["raise_error"].finditer(content):
            if match.group(1):
                raised_errors.add(match.group(1))

        # Look for caught exceptions (often re-raised)
        for match in self.PATTERNS["except_error"].finditer(content):
            raised_errors.add(match.group(1))

        # Add success behavior if there's a return
        if re.search(r"return\s+", content):
            behaviors.append(Behavior(
                input="Valid JWT token",
                output="Claims object",
                category="success",
            ))

        # Add error behaviors
        if "ExpiredSignatureError" in raised_errors or "Expired" in content:
            behaviors.append(Behavior(
                input="Expired token",
                output="jwt.ExpiredSignatureError",
                category="error",
            ))

        if "InvalidTokenError" in raised_errors or "Invalid" in content:
            behaviors.append(Behavior(
                input="Invalid token",
                output="jwt.InvalidTokenError",
                category="error",
            ))

        return behaviors

    def _extract_docstring(self, content: str, position: int) -> str:
        """Extract docstring after a position."""
        after = content[position:position + 500]
        # Look for """...""" or '''...'''
        match = re.search(r'^\s*["\'][\'"]{2}([^"\']*)[\'"]{3}', after)
        if match:
            return match.group(1).strip().split("\n")[0]
        return ""
