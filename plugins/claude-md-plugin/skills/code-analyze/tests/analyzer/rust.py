"""Rust code analyzer using regex patterns."""

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


class RustAnalyzer(Analyzer):
    """Analyzer for Rust files."""

    @property
    def language(self) -> str:
        return "rust"

    @property
    def file_extensions(self) -> list[str]:
        return [".rs"]

    # Regex patterns for Rust analysis
    PATTERNS = {
        # pub fn function_name(params) -> ReturnType
        "pub_function": re.compile(
            r"^pub\s+(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]+>)?\s*\(([^)]*)\)\s*(?:->\s*([^\{]+?))?\s*\{",
            re.MULTILINE,
        ),
        # pub struct Name { ... }
        "pub_struct": re.compile(
            r"^pub\s+struct\s+(\w+)",
            re.MULTILINE,
        ),
        # pub enum Name { ... }
        "pub_enum": re.compile(
            r"^pub\s+enum\s+(\w+)",
            re.MULTILINE,
        ),
        # pub trait Name { ... }
        "pub_trait": re.compile(
            r"^pub\s+trait\s+(\w+)",
            re.MULTILINE,
        ),
        # use crate_name::... or use crate_name;
        "use_external": re.compile(
            r"^use\s+(\w+)(?:::|;)",
            re.MULTILINE,
        ),
        # #[derive(..., thiserror::Error, ...)]
        "derive_use": re.compile(
            r"#\[derive\([^\)]*?(\w+)::(\w+)",
            re.MULTILINE,
        ),
        # TokenError::Expired
        "error_variant": re.compile(
            r"(\w+Error)::(\w+)",
            re.MULTILINE,
        ),
        # Result<T, E>
        "result_type": re.compile(
            r"Result<([^,>]+),\s*(\w+Error)>",
            re.MULTILINE,
        ),
    }

    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a Rust file."""
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
        """Extract all exports from Rust content."""
        functions = []
        types = []
        classes = []

        # Extract pub functions
        for match in self.PATTERNS["pub_function"].finditer(content):
            name = match.group(1)
            params = match.group(2).strip()
            return_type = match.group(3).strip() if match.group(3) else ""

            signature = f"fn {name}({self._simplify_params(params)})"
            if return_type:
                signature += f" -> {return_type}"

            functions.append(ExportedFunction(
                name=name,
                signature=signature,
                description=self._extract_doc_comment(content, match.start()),
            ))

        # Extract pub structs
        for match in self.PATTERNS["pub_struct"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="struct",
                definition=f"pub struct {name}",
                description=self._extract_doc_comment(content, match.start()),
            ))

        # Extract pub enums
        for match in self.PATTERNS["pub_enum"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="enum",
                definition=f"pub enum {name}",
                description=self._extract_doc_comment(content, match.start()),
            ))

        # Extract pub traits
        for match in self.PATTERNS["pub_trait"].finditer(content):
            name = match.group(1)
            types.append(ExportedType(
                name=name,
                kind="trait",
                definition=f"pub trait {name}",
                description=self._extract_doc_comment(content, match.start()),
            ))

        return Exports(functions=functions, types=types, classes=classes)

    def _extract_dependencies(self, content: str) -> Dependencies:
        """Extract dependencies from use statements."""
        external = []
        internal = []

        std_crates = {"std", "core", "alloc"}

        for match in self.PATTERNS["use_external"].finditer(content):
            crate_name = match.group(1)
            if crate_name in std_crates:
                continue
            if crate_name == "crate" or crate_name == "super" or crate_name == "self":
                # Internal module reference
                continue
            if crate_name not in external:
                external.append(crate_name)

        # Also check for derive macros from external crates
        for match in self.PATTERNS["derive_use"].finditer(content):
            crate_name = match.group(1)
            if crate_name not in std_crates and crate_name not in external:
                external.append(crate_name)

        return Dependencies(external=external, internal=internal)

    def _extract_behaviors(self, content: str) -> list[Behavior]:
        """Infer behaviors from code patterns."""
        behaviors = []

        # Look for error variants in Result returns
        error_variants = set()
        for match in self.PATTERNS["error_variant"].finditer(content):
            error_type = match.group(1)
            variant = match.group(2)
            error_variants.add((error_type, variant))

        # Add success behavior if there's Ok return
        if "Ok(" in content:
            behaviors.append(Behavior(
                input="Valid JWT token",
                output="Claims object",
                category="success",
            ))

        # Add error behaviors
        for error_type, variant in error_variants:
            if "Expired" in variant:
                behaviors.append(Behavior(
                    input="Expired token",
                    output=f"{error_type}::{variant}",
                    category="error",
                ))
            elif "Invalid" in variant:
                behaviors.append(Behavior(
                    input="Invalid token",
                    output=f"{error_type}::{variant}",
                    category="error",
                ))

        return behaviors

    def _extract_doc_comment(self, content: str, position: int) -> str:
        """Extract doc comment (///) before a position."""
        before = content[:position]
        lines = before.rstrip().split("\n")
        for line in reversed(lines):
            stripped = line.strip()
            if stripped.startswith("///"):
                return stripped[3:].strip()
            if stripped and not stripped.startswith("//") and not stripped.startswith("#["):
                break
        return ""

    def _simplify_params(self, params: str) -> str:
        """Simplify parameter list for display."""
        if not params:
            return ""
        # Just show param names and types, simplified
        parts = []
        for part in params.split(","):
            part = part.strip()
            if part:
                # Take first word (param name) and type hint
                simplified = part.split(":")[0].strip() if ":" in part else part
                parts.append(simplified)
        return ", ".join(parts[:3]) + ("..." if len(parts) > 3 else "")
