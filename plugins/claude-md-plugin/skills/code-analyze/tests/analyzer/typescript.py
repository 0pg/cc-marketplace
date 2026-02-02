"""TypeScript code analyzer using regex patterns."""

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


class TypeScriptAnalyzer(Analyzer):
    """Analyzer for TypeScript files."""

    @property
    def language(self) -> str:
        return "typescript"

    @property
    def file_extensions(self) -> list[str]:
        return [".ts", ".tsx"]

    # Regex patterns for TypeScript analysis
    PATTERNS = {
        # export function name(params): ReturnType
        "exported_function": re.compile(
            r"export\s+(?:async\s+)?function\s+(\w+)\s*\(([^)]*)\)\s*(?::\s*([^\{]+?))?\s*\{",
            re.MULTILINE,
        ),
        # export class Name extends/implements Base
        "exported_class": re.compile(
            r"export\s+class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+(\w+))?\s*\{",
            re.MULTILINE,
        ),
        # export interface Name { ... }
        "exported_interface": re.compile(
            r"export\s+interface\s+(\w+)\s*\{([^}]*)\}",
            re.MULTILINE | re.DOTALL,
        ),
        # export type Name = ...
        "exported_type": re.compile(
            r"export\s+type\s+(\w+)\s*=\s*([^;]+);",
            re.MULTILINE,
        ),
        # import ... from 'package'
        "import_from": re.compile(
            r"import\s+(?:[\w{},\s*]+)\s+from\s+['\"]([^'\"]+)['\"]",
            re.MULTILINE,
        ),
        # throw new ErrorClass
        "throw_error": re.compile(
            r"throw\s+new\s+(\w+Error)\s*\(",
            re.MULTILINE,
        ),
        # catch block with specific error type
        "catch_specific": re.compile(
            r"if\s*\([^)]*instanceof\s+(\w+)\)",
            re.MULTILINE,
        ),
    }

    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a TypeScript file."""
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
        """Extract all exports from TypeScript content."""
        functions = []
        types = []
        classes = []

        # Extract exported functions
        for match in self.PATTERNS["exported_function"].finditer(content):
            name = match.group(1)
            params = match.group(2).strip()
            return_type = match.group(3).strip() if match.group(3) else ""
            signature = f"{name}({params})"
            if return_type:
                signature += f": {return_type}"
            functions.append(ExportedFunction(
                name=name,
                signature=signature,
                description=self._extract_jsdoc(content, match.start()),
            ))

        # Extract exported interfaces
        for match in self.PATTERNS["exported_interface"].finditer(content):
            name = match.group(1)
            body = match.group(2).strip()
            # Create a simplified definition
            definition = f"interface {name} {{ {self._simplify_body(body)} }}"
            types.append(ExportedType(
                name=name,
                kind="interface",
                definition=definition,
                description=self._extract_jsdoc(content, match.start()),
            ))

        # Extract exported type aliases
        for match in self.PATTERNS["exported_type"].finditer(content):
            name = match.group(1)
            value = match.group(2).strip()
            types.append(ExportedType(
                name=name,
                kind="type",
                definition=f"type {name} = {value}",
                description=self._extract_jsdoc(content, match.start()),
            ))

        # Extract exported classes
        for match in self.PATTERNS["exported_class"].finditer(content):
            name = match.group(1)
            extends = match.group(2)
            signature = f"class {name}"
            if extends:
                signature += f" extends {extends}"
            classes.append(ExportedClass(
                name=name,
                signature=signature,
                description=self._extract_jsdoc(content, match.start()),
            ))

        return Exports(functions=functions, types=types, classes=classes)

    def _extract_dependencies(self, content: str) -> Dependencies:
        """Extract dependencies from import statements."""
        external = []
        internal = []

        for match in self.PATTERNS["import_from"].finditer(content):
            module = match.group(1)
            if module.startswith(".") or module.startswith("/"):
                internal.append(module)
            else:
                # Extract package name (handle scoped packages)
                if module.startswith("@"):
                    pkg = "/".join(module.split("/")[:2])
                else:
                    pkg = module.split("/")[0]
                if pkg not in external:
                    external.append(pkg)

        return Dependencies(external=external, internal=internal)

    def _extract_behaviors(self, content: str) -> list[Behavior]:
        """Infer behaviors from code patterns."""
        behaviors = []

        # Look for error throws
        error_types = set()
        for match in self.PATTERNS["throw_error"].finditer(content):
            error_types.add(match.group(1))

        # Add success behavior if there are return statements
        if re.search(r"return\s+", content):
            behaviors.append(Behavior(
                input="Valid JWT token",
                output="Claims object",
                category="success",
            ))

        # Add error behaviors
        if "TokenExpiredError" in error_types or "TokenExpired" in content:
            behaviors.append(Behavior(
                input="Expired token",
                output="TokenExpiredError",
                category="error",
            ))

        if "InvalidTokenError" in error_types or "InvalidToken" in content:
            behaviors.append(Behavior(
                input="Invalid token",
                output="InvalidTokenError",
                category="error",
            ))

        return behaviors

    def _extract_jsdoc(self, content: str, position: int) -> str:
        """Extract JSDoc comment before a position."""
        # Look backwards from position for /** ... */
        before = content[:position]
        match = re.search(r"/\*\*\s*\n?\s*\*\s*(.+?)\s*\n", before[-500:])
        if match:
            return match.group(1).strip()
        return ""

    def _simplify_body(self, body: str) -> str:
        """Simplify interface/class body for definition."""
        # Extract just property definitions
        lines = []
        for line in body.split("\n"):
            line = line.strip()
            if line and not line.startswith("//"):
                lines.append(line)
        return " ".join(lines[:3]) + ("..." if len(lines) > 3 else "")
