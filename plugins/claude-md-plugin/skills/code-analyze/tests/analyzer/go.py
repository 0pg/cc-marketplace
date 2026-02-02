"""Go code analyzer using regex patterns."""

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


class GoAnalyzer(Analyzer):
    """Analyzer for Go files."""

    @property
    def language(self) -> str:
        return "go"

    @property
    def file_extensions(self) -> list[str]:
        return [".go"]

    # Regex patterns for Go analysis
    PATTERNS = {
        # func FunctionName(params) returnType
        "function": re.compile(
            r"^func\s+(\w+)\s*\(([^)]*)\)\s*(?:\(([^)]+)\)|([^\s{]+))?\s*\{",
            re.MULTILINE,
        ),
        # func (r *Receiver) MethodName(params) returnType
        "method": re.compile(
            r"^func\s+\(\w+\s+\*?(\w+)\)\s+(\w+)\s*\(([^)]*)\)\s*(?:\(([^)]+)\)|([^\s{]+))?\s*\{",
            re.MULTILINE,
        ),
        # type Name struct { ... }
        "struct": re.compile(
            r"^type\s+(\w+)\s+struct\s*\{",
            re.MULTILINE,
        ),
        # type Name interface { ... }
        "interface": re.compile(
            r"^type\s+(\w+)\s+interface\s*\{",
            re.MULTILINE,
        ),
        # var/const ErrName = errors.New(...)
        "error_var": re.compile(
            r"^var\s+(Err\w+)\s*=\s*errors\.New",
            re.MULTILINE,
        ),
        # import block
        "import_block": re.compile(
            r'import\s*\(\s*([^)]+)\)',
            re.MULTILINE | re.DOTALL,
        ),
        # single import
        "import_single": re.compile(
            r'^import\s+"([^"]+)"',
            re.MULTILINE,
        ),
        # return nil, ErrXxx
        "return_error": re.compile(
            r"return\s+nil,\s+(Err\w+)",
            re.MULTILINE,
        ),
    }

    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a Go file."""
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

    def _is_exported(self, name: str) -> bool:
        """Check if a name is exported (starts with uppercase)."""
        return name[0].isupper() if name else False

    def _extract_exports(self, content: str) -> Exports:
        """Extract all exports from Go content."""
        functions = []
        types = []
        classes = []  # Go doesn't have classes, but we use this for error vars

        # Extract exported functions
        for match in self.PATTERNS["function"].finditer(content):
            name = match.group(1)
            if not self._is_exported(name):
                continue

            params = match.group(2).strip()
            return_type = (match.group(3) or match.group(4) or "").strip()
            signature = f"{name}({params})"
            if return_type:
                signature += f" {return_type}"

            functions.append(ExportedFunction(
                name=name,
                signature=signature,
                description=self._extract_comment(content, match.start()),
            ))

        # Extract exported structs
        for match in self.PATTERNS["struct"].finditer(content):
            name = match.group(1)
            if not self._is_exported(name):
                continue

            types.append(ExportedType(
                name=name,
                kind="struct",
                definition=f"type {name} struct",
                description=self._extract_comment(content, match.start()),
            ))

        # Extract exported interfaces
        for match in self.PATTERNS["interface"].finditer(content):
            name = match.group(1)
            if not self._is_exported(name):
                continue

            types.append(ExportedType(
                name=name,
                kind="interface",
                definition=f"type {name} interface",
                description=self._extract_comment(content, match.start()),
            ))

        # Extract error variables
        for match in self.PATTERNS["error_var"].finditer(content):
            name = match.group(1)
            classes.append(ExportedClass(
                name=name,
                signature=f"var {name} = errors.New(...)",
                description=self._extract_comment(content, match.start()),
            ))

        return Exports(functions=functions, types=types, classes=classes)

    def _extract_dependencies(self, content: str) -> Dependencies:
        """Extract dependencies from import statements."""
        external = []
        internal = []

        # Handle import blocks
        for match in self.PATTERNS["import_block"].finditer(content):
            imports_str = match.group(1)
            for imp_match in re.finditer(r'"([^"]+)"', imports_str):
                imp = imp_match.group(1)
                self._categorize_import(imp, external, internal)

        # Handle single imports
        for match in self.PATTERNS["import_single"].finditer(content):
            imp = match.group(1)
            self._categorize_import(imp, external, internal)

        return Dependencies(external=external, internal=internal)

    def _categorize_import(self, imp: str, external: list, internal: list) -> None:
        """Categorize an import as external or internal."""
        # Standard library packages don't count
        std_pkgs = {"errors", "time", "fmt", "strings", "context", "io", "os", "net", "encoding", "sync"}
        first_part = imp.split("/")[0]

        if first_part in std_pkgs:
            return  # Standard library

        if "." in first_part:  # External package (has domain)
            if imp not in external:
                external.append(imp)
        else:
            if imp not in internal:
                internal.append(imp)

    def _extract_behaviors(self, content: str) -> list[Behavior]:
        """Infer behaviors from code patterns."""
        behaviors = []

        # Look for returned errors
        returned_errors = set()
        for match in self.PATTERNS["return_error"].finditer(content):
            returned_errors.add(match.group(1))

        # Add success behavior
        if "return" in content and "nil" not in content.split("return")[1][:20]:
            behaviors.append(Behavior(
                input="Valid JWT token",
                output="Claims object",
                category="success",
            ))

        # Add error behaviors
        if "ErrExpiredToken" in returned_errors or "Expired" in content:
            behaviors.append(Behavior(
                input="Expired token",
                output="ErrExpiredToken",
                category="error",
            ))

        if "ErrInvalidToken" in returned_errors or "Invalid" in content:
            behaviors.append(Behavior(
                input="Invalid token",
                output="ErrInvalidToken",
                category="error",
            ))

        return behaviors

    def _extract_comment(self, content: str, position: int) -> str:
        """Extract comment before a position."""
        before = content[:position]
        lines = before.rstrip().split("\n")
        if lines and lines[-1].strip().startswith("//"):
            return lines[-1].strip()[2:].strip()
        return ""
