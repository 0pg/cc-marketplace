"""Base analyzer interface and result types."""

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class ExportedFunction:
    """Represents an exported function."""
    name: str
    signature: str = ""
    description: str = ""


@dataclass
class ExportedType:
    """Represents an exported type (interface, type alias, struct, etc.)."""
    name: str
    kind: str = ""  # interface, type, struct, enum, etc.
    definition: str = ""
    description: str = ""


@dataclass
class ExportedClass:
    """Represents an exported class."""
    name: str
    signature: str = ""
    description: str = ""


@dataclass
class Behavior:
    """Represents an inferred behavior."""
    input: str
    output: str
    category: str = "success"  # success or error


@dataclass
class Exports:
    """Container for all exports."""
    functions: list[ExportedFunction] = field(default_factory=list)
    types: list[ExportedType] = field(default_factory=list)
    classes: list[ExportedClass] = field(default_factory=list)


@dataclass
class Dependencies:
    """Container for dependencies."""
    external: list[str] = field(default_factory=list)
    internal: list[str] = field(default_factory=list)


@dataclass
class AnalysisResult:
    """Complete analysis result for a file or directory."""
    path: str = ""
    exports: Exports = field(default_factory=Exports)
    dependencies: Dependencies = field(default_factory=Dependencies)
    behaviors: list[Behavior] = field(default_factory=list)
    analyzed_files: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON serialization."""
        return {
            "path": self.path,
            "exports": {
                "functions": [
                    {"name": f.name, "signature": f.signature, "description": f.description}
                    for f in self.exports.functions
                ],
                "types": [
                    {"name": t.name, "kind": t.kind, "definition": t.definition, "description": t.description}
                    for t in self.exports.types
                ],
                "classes": [
                    {"name": c.name, "signature": c.signature, "description": c.description}
                    for c in self.exports.classes
                ],
            },
            "dependencies": {
                "external": self.dependencies.external,
                "internal": self.dependencies.internal,
            },
            "behaviors": [
                {"input": b.input, "output": b.output, "category": b.category}
                for b in self.behaviors
            ],
            "analyzed_files": self.analyzed_files,
        }


class Analyzer(ABC):
    """Abstract base class for language-specific analyzers."""

    @property
    @abstractmethod
    def language(self) -> str:
        """Return the language name."""
        pass

    @property
    @abstractmethod
    def file_extensions(self) -> list[str]:
        """Return list of file extensions for this language."""
        pass

    @abstractmethod
    def analyze_file(self, file_path: Path) -> AnalysisResult:
        """Analyze a single file."""
        pass

    def analyze_directory(self, dir_path: Path, files: Optional[list[str]] = None) -> AnalysisResult:
        """Analyze a directory, optionally limiting to specific files."""
        result = AnalysisResult(path=str(dir_path))

        if files:
            target_files = [dir_path / f for f in files]
        else:
            target_files = [
                f for f in dir_path.iterdir()
                if f.is_file() and f.suffix in self.file_extensions
            ]

        for file_path in target_files:
            if not file_path.exists():
                continue

            file_result = self.analyze_file(file_path)

            # Merge exports
            result.exports.functions.extend(file_result.exports.functions)
            result.exports.types.extend(file_result.exports.types)
            result.exports.classes.extend(file_result.exports.classes)

            # Merge dependencies (deduplicate)
            for dep in file_result.dependencies.external:
                if dep not in result.dependencies.external:
                    result.dependencies.external.append(dep)
            for dep in file_result.dependencies.internal:
                if dep not in result.dependencies.internal:
                    result.dependencies.internal.append(dep)

            # Merge behaviors
            result.behaviors.extend(file_result.behaviors)

            # Track analyzed files
            result.analyzed_files.append(file_path.name)

        return result

    def read_file_content(self, file_path: Path) -> str:
        """Read file content safely."""
        try:
            return file_path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            return ""
