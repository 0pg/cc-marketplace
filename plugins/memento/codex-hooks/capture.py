#!/usr/bin/env python3
"""Bounded local Codex hook adapter; checkpoint state belongs to the Rust CLI."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import shlex
import subprocess
import sys
import tempfile
import time

INPUT_LIMIT = 256 * 1024
CONFIG_LIMIT = 64 * 1024
OUTPUT_LIMIT = 16 * 1024
DETAIL_LIMIT = 2048
PROJECT_LIMIT = 64
CONTINUATION_PREFIX = "[memento checkpoint continuation] "
SUPPORTED_EVENTS = {
    "SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse",
    "Stop", "PreCompact", "Interrupt", "SessionEnd",
}
SELF_COMMANDS = {"memento", "memento.exe", "memento.py", "capture.py"}
WRITE_COMMANDS = {"mkdir", "rmdir", "rm", "mv", "cp", "touch", "tee", "install", "truncate"}
GIT_WRITES = {"add", "restore", "reset", "checkout", "switch", "cherry-pick", "revert", "merge", "rebase", "apply", "clean", "stash"}


class CaptureError(Exception):
    """Expected configuration, protocol or persistence failure."""


def bounded_json(path, limit):
    with path.open("rb") as stream:
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise CaptureError("capture configuration exceeds its size limit")
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise CaptureError("capture configuration must be an object")
    return value


def absolute_path(value, label):
    if not isinstance(value, str) or not value or not Path(value).is_absolute():
        raise CaptureError(f"{label} must be an absolute path")
    return Path(value).resolve()


def identifier(value, label):
    if (not isinstance(value, str) or not value.strip() or len(value) > 256
            or len(value.encode()) > 512 or any(ord(character) < 32 or 127 <= ord(character) <= 159 for character in value)):
        raise CaptureError(f"{label} must be a nonempty identifier within 256 characters and 512 bytes, without controls")
    return value


def bounded_process(command, input_bytes, timeout, failure_message):
    """Terminate excess output without writing raw subprocess output to disk."""
    output = bytearray()
    with subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                          stderr=subprocess.DEVNULL) as process:
        deadline = time.monotonic() + timeout
        written = 0
        os.set_blocking(process.stdin.fileno(), False)
        os.set_blocking(process.stdout.fileno(), False)
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(process.stdout, selectors.EVENT_READ)
                if input_bytes:
                    selector.register(process.stdin, selectors.EVENT_WRITE)
                else:
                    process.stdin.close()
                while selector.get_map():
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise CaptureError("checkpoint process timed out")
                    for key, _ in selector.select(remaining):
                        if key.fileobj is process.stdin:
                            try:
                                written += os.write(process.stdin.fileno(), input_bytes[written:written + 4096])
                            except BrokenPipeError:
                                written = len(input_bytes)
                            if written == len(input_bytes):
                                selector.unregister(process.stdin)
                                process.stdin.close()
                        else:
                            chunk = os.read(process.stdout.fileno(), 1024)
                            if not chunk:
                                selector.unregister(process.stdout)
                                continue
                            output.extend(chunk)
                            if len(output) > OUTPUT_LIMIT:
                                raise CaptureError("checkpoint response exceeds its size limit")
            process.wait(timeout=max(deadline - time.monotonic(), 0.01))
        except (CaptureError, OSError, subprocess.TimeoutExpired):
            process.kill()
            process.wait()
            raise
        if process.returncode != 0:
            raise CaptureError(failure_message)
    return bytes(output)


def repository_root(cwd):
    directory = absolute_path(cwd, "cwd")
    result = subprocess.run(
        ["git", "-C", str(directory), "rev-parse", "--show-toplevel"],
        capture_output=True, text=True, timeout=0.5, check=False,
    )
    if result.returncode != 0:
        return None
    return absolute_path(result.stdout.strip(), "repository")


def read_config(path):
    config = bounded_json(path, CONFIG_LIMIT)
    projects = config.get("projects")
    if type(config.get("version")) is not int or config["version"] != 1 or not isinstance(projects, list) or len(projects) > PROJECT_LIMIT:
        raise CaptureError("capture configuration needs version 1 and at most 64 projects")
    seen = set()
    for project in projects:
        if not isinstance(project, dict):
            raise CaptureError("project mapping must be an object")
        repository = absolute_path(project.get("repository"), "repository")
        absolute_path(project.get("store"), "store")
        if project.get("policy") is not None:
            absolute_path(project["policy"], "policy")
        identifier(project.get("project_id"), "project_id")
        identifier(project.get("work_id"), "work_id")
        if str(repository) in seen:
            raise CaptureError("repository mappings must be unique")
        seen.add(str(repository))
    return config


def command_words(payload, allow_compound=False):
    tool_input = payload.get("tool_input", {})
    if not isinstance(tool_input, dict):
        return []
    command = tool_input.get("command", tool_input.get("cmd", ""))
    if not isinstance(command, str) or len(command) > 16 * 1024:
        return []
    try:
        lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|<>")
        lexer.whitespace_split = True
        words = list(lexer)
    except ValueError:
        return []
    # This deliberately handles simple direct commands, not arbitrary shell programs.
    if not allow_compound and any(word and set(word).issubset(set(";&|")) for word in words):
        return []
    while words and "=" in words[0] and not words[0].startswith(("/", "-")):
        words = words[1:]
    if words and Path(words[0]).name == "env":
        words = words[1:]
        while words and "=" in words[0] and not words[0].startswith(("/", "-")):
            words = words[1:]
    return words


def is_self(words):
    if not words:
        return False
    first = Path(words[0]).name
    if first in SELF_COMMANDS:
        return True
    return first.startswith("python") and len(words) > 1 and Path(words[1]).name in SELF_COMMANDS


def git_subcommand(words):
    if not words or Path(words[0]).name != "git":
        return None
    position = 1
    while position < len(words):
        word = words[position]
        if word in {"-C", "-c", "--git-dir", "--work-tree"}:
            position += 2
        elif word.startswith("-"):
            position += 1
        else:
            return word
    return None


def commit_repository_matches(payload, words, repository):
    """A native command gate supports direct Git commits in this worktree only."""
    tool_input = payload.get("tool_input", {})
    command = tool_input.get("command", tool_input.get("cmd", ""))
    original = shlex.split(command)
    if any(word.startswith("GIT_") and "=" in word for word in original):
        return False
    directory = Path(payload["cwd"])
    position = 1
    while position < len(words):
        word = words[position]
        if word == "-C":
            if position + 1 >= len(words):
                return False
            directory = (directory / words[position + 1]).resolve()
            position += 2
        elif word.startswith("-"):
            return False  # Only -C is part of this deliberately narrow native gate.
        else:
            break
    return repository_root(str(directory)) == Path(repository)


def is_write(payload, words):
    if payload.get("tool_name") in {"apply_patch", "Edit", "Write"}:
        return True
    if not words:
        return False
    executable = Path(words[0]).name
    if executable in WRITE_COMMANDS or git_subcommand(words) in GIT_WRITES:
        return True
    # Recognizable output redirection is a write; shell syntax is not interpreted.
    return any(word in {">", ">>", "1>", "1>>"} for word in words)


def is_verification(words):
    if not words:
        return False
    executable = Path(words[0]).name
    if executable in {"pytest", "pytest.exe"}:
        return True
    if len(words) < 2:
        return False
    if executable == "cargo":
        return words[1] in {"test", "clippy", "check"} or (words[1] == "fmt" and "--check" in words)
    if executable.startswith("python"):
        return words[1:3] in (["-m", "pytest"], ["-m", "unittest"])
    return executable in {"npm", "pnpm", "yarn"} and words[1] in {"test", "check", "lint", "typecheck"}


def response_value(payload):
    response = payload.get("tool_response")
    if isinstance(response, str):
        try:
            return json.loads(response)
        except ValueError:
            return response
    return response


def failed(response):
    if not isinstance(response, dict):
        return False
    if response.get("isError") is True or response.get("is_error") is True:
        return True
    exit_code = response.get("exit_code")
    return isinstance(exit_code, int) and not isinstance(exit_code, bool) and exit_code != 0


def detail(payload, response=None):
    tool_input = payload.get("tool_input", {})
    command = tool_input.get("command", tool_input.get("cmd", "")) if isinstance(tool_input, dict) else ""
    header = f"Observed tool {payload.get('tool_name', '')}: {command}"[:512]
    suffix = json.dumps(response, ensure_ascii=False, separators=(",", ":")) if response is not None else ""
    rendered = header + "\n" + suffix
    if len(rendered) > DETAIL_LIMIT:
        return rendered[:DETAIL_LIMIT - 64] + "\n[capture detail truncated; not the complete tool output]"
    return rendered


class Dispatcher:
    def __init__(self, payload, plugin_root, plugin_data):
        self.payload = payload
        self.root = plugin_root
        self.data = plugin_data
        self.project = None
        self.scope = None
        self.timeout = 1.5 if payload.get("hook_event_name") in {"Interrupt", "SessionEnd"} else 10

    def select(self):
        path = self.data / "capture-config.json"
        if not path.is_file():
            return False
        config = read_config(path)
        repository = repository_root(self.payload.get("cwd"))
        if repository is None:
            return False
        for project in config["projects"]:
            if absolute_path(project["repository"], "repository") == repository:
                self.project = project
                break
        if self.project is None:
            return False
        session = identifier(self.payload.get("session_id"), "session_id")
        turn = self.payload.get("turn_id")
        if turn is None and self.payload.get("hook_event_name") == "SessionStart":
            turn = "session-start"
        self.scope = {
            "project_id": self.project["project_id"], "repository": str(repository),
            "work_id": self.project["work_id"], "session_id": session,
            "turn_id": identifier(turn, "turn_id"),
        }
        return True

    def call(self, operation, **fields):
        binary = self.root / "skills/memento/bin/memento"
        request = {"operation": operation, "scope": self.scope, **fields}
        command = [str(binary), "checkpoint", "--store", self.project["store"], "--summary", "true", "--input", "-"]
        if self.project.get("policy") is not None:
            command.extend(["--policy", self.project["policy"]])
        raw = bounded_process(
            command,
            json.dumps(request, ensure_ascii=False).encode(), self.timeout,
            "checkpoint persistence or validation failed",
        )
        reply = json.loads(raw)
        if not isinstance(reply, dict) or reply.get("durable") is not True:
            raise CaptureError("checkpoint did not return a durable acknowledgement")
        if reply.get("decision") not in {"allow", "block", "capture_incomplete"}:
            raise CaptureError("checkpoint did not return a recognized decision")
        for name in ("pending_event_ids", "capture_incomplete_event_ids"):
            values = reply.get(name)
            if not isinstance(values, list) or len(values) > 8 or any(not isinstance(value, str) for value in values):
                raise CaptureError("checkpoint did not return bounded checkpoint identifiers")
        if not isinstance(reply.get("pending_user_prompt"), bool):
            raise CaptureError("checkpoint status did not identify pending user requests")
        return reply

    def open(self, kind, text):
        native = self.payload.get("tool_use_id") or self.payload.get("turn_id") or "session"
        identity = [self.scope["project_id"], self.scope["repository"], self.scope["work_id"],
                    self.scope["session_id"], self.payload["hook_event_name"], native]
        if kind == "user_prompt":
            identity.append(hashlib.sha256(text.encode()).hexdigest())
        event_id = "codex:" + hashlib.sha256(json.dumps(identity).encode()).hexdigest()
        if kind == "user_prompt" and len(text) > DETAIL_LIMIT:
            marker = "\n[prompt truncated; not complete user message]"
            text = text[:DETAIL_LIMIT - len(marker)] + marker
        return self.call("open", event_id=event_id, kind=kind, detail=text[:DETAIL_LIMIT])

    def context(self, reply):
        pending = reply.get("pending_event_ids", [])
        scope = json.dumps(self.scope, ensure_ascii=False, separators=(",", ":"))
        text = (
            f"Use Memento for this configured project. Read {self.root / 'skills/memento/SKILL.md'}. "
            f"Store: {self.project['store']}. Checkpoint scope: {scope}. "
            f"Pending checkpoint IDs: {json.dumps(pending[:8], ensure_ascii=False)}. "
            f"Additional pending IDs omitted: {reply.get('omitted_pending_event_ids', 0)}. "
            f"Checkpoint result: {reply.get('reason', reply['decision'])}. "
            "Inspect full checkpoint status when resolving all pending events. "
            "Before the first change, record the request and constraints. Record corrections, decisions, "
            "failed or rejected attempts, verification limits and handoff using accessible evidence. "
            "Resolve each checkpoint with its exact persisted record/source/revision/sequence and durable receipt. "
            "Use no_new_context only when no meaningful new context exists; report capture_incomplete on failure."
        )
        return text.encode()[:6000].decode(errors="ignore")

    def gate(self, operation, event):
        try:
            reply = self.call(operation)
        except (CaptureError, OSError, ValueError, TypeError, RecursionError, subprocess.SubprocessError) as error:
            return self.denied(event, f"memento capture_incomplete: cannot verify this checkpoint: {str(error)[:256]}")
        blocked = reply["decision"] != "allow" if operation == "check_commit" else reply["pending_user_prompt"]
        return self.denied(event, self.context(reply)) if blocked else {}

    @staticmethod
    def denied(event, reason):
        return {"hookSpecificOutput": {"hookEventName": event, "permissionDecision": "deny",
                "permissionDecisionReason": reason}}

    def run(self):
        event = self.payload.get("hook_event_name")
        if event not in SUPPORTED_EVENTS:
            return {}
        if event == "SessionStart" and not (self.data / "capture-config.json").exists():
            return self.additional(event,
                f"Memento hooks need explicit project configuration at {self.data / 'capture-config.json'}. "
                f"Read {self.root / 'skills/memento/SKILL.md'} and use "
                f"python3 {shlex.quote(str(self.root / 'codex-hooks/capture.py'))} configure "
                f"--data-dir {shlex.quote(str(self.data))} "
                "--repository ABS --store ABS --project-id ID --work-id ID. Hook trust must be reviewed in Codex.")
        if not self.select():
            return {}
        if event == "SessionStart":
            return self.additional(event, self.context(self.call("status")))
        if event == "UserPromptSubmit":
            prompt = self.payload.get("prompt")
            if not isinstance(prompt, str):
                raise CaptureError("UserPromptSubmit requires a textual prompt")
            reply = self.call("status") if prompt.startswith(CONTINUATION_PREFIX) else self.open("user_prompt", prompt)
            return self.additional(event, self.context(reply))
        if event in {"PreToolUse", "PostToolUse"}:
            words = command_words(self.payload)
            if is_self(command_words(self.payload, allow_compound=True)):
                return {}
            git_command = git_subcommand(words)
            if event == "PreToolUse":
                if git_command == "commit":
                    if not commit_repository_matches(self.payload, words, self.scope["repository"]):
                        return self.denied(event, "Memento could not verify this Git command's repository or "
                            "index context. Run a direct commit in the configured worktree; Git environment, "
                            "configuration and repository overrides need the actual Git checkpoint gate.")
                    return self.gate("check_commit", event)
                elif is_write(self.payload, words):
                    return self.gate("status", event)
                return {}
            response = response_value(self.payload)
            if isinstance(response, dict) and response.get("session_id") and response.get("exit_code") is None:
                return {}  # Still-running unified exec, not an observed completion.
            if failed(response):
                kind = "tool_failure"
            elif is_verification(words):
                kind = "verification"
            elif is_write(self.payload, words):
                kind = "mutation"
            else:
                return {}
            reply = self.open(kind, detail(self.payload, response))
            return self.additional(event, self.context(reply))
        if event == "Stop":
            reply = self.call("stop")
            if reply.get("decision") == "block":
                return {"decision": "block", "reason": CONTINUATION_PREFIX + self.context(reply)}
            if reply.get("decision") == "capture_incomplete" or reply.get("capture_incomplete_event_ids"):
                return {"systemMessage": "memento capture_incomplete: checkpoint capture remains incomplete; "
                        "saved records are not a complete account of this work."}
            return {}
        if event == "PreCompact":
            reply = self.call("status")
            if reply.get("pending_event_ids"):
                return {"continue": False, "stopReason": "memento capture_incomplete: pending checkpoints "
                        "must be saved before Codex compaction; this hook does not generate records."}
            if reply.get("capture_incomplete_event_ids") or reply.get("decision") == "capture_incomplete":
                return {"systemMessage": "memento capture_incomplete: acknowledged capture gaps remain "
                        "before compaction; this hook does not generate missing records."}
            return {}
        if event == "Interrupt":
            self.open("interrupt", "The user interrupted this Codex turn; completeness is not established.")
            return {"systemMessage": "memento: interruption observed; unfinished checkpoints remain pending."}
        reply = self.call("status")  # SessionEnd is advisory, not a final model pass.
        if reply.get("decision") != "allow" or reply.get("capture_incomplete_event_ids"):
            return {"systemMessage": "memento capture_incomplete: session ended with unfinished checkpoints."}
        return {}

    @staticmethod
    def additional(event, text):
        bounded = text.encode()[:6000].decode(errors="ignore")
        return {"hookSpecificOutput": {"hookEventName": event, "additionalContext": bounded}}


def configure(arguments):
    parser = argparse.ArgumentParser(description="Explicitly configure a local repository for Memento hooks.")
    parser.add_argument("--data-dir", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--store", required=True)
    parser.add_argument("--project-id", required=True)
    parser.add_argument("--work-id", required=True)
    parser.add_argument("--policy")
    parser.add_argument("--initialize-store", action="store_true")
    args = parser.parse_args(arguments)
    data = absolute_path(args.data_dir, "data-dir")
    requested_repository = absolute_path(args.repository, "repository")
    repository = repository_root(str(requested_repository))
    if repository is None or repository != requested_repository:
        raise CaptureError("repository must be the canonical Git working-tree root")
    project = {
        "repository": str(repository), "store": str(absolute_path(args.store, "store")),
        "project_id": identifier(args.project_id, "project-id"), "work_id": identifier(args.work_id, "work-id"),
    }
    destination = data / "capture-config.json"
    config = read_config(destination) if destination.exists() else {"version": 1, "projects": []}
    existing_project = next((existing for existing in config["projects"]
                             if absolute_path(existing["repository"], "repository") == repository), None)
    policy = args.policy if args.policy is not None else (existing_project or {}).get("policy")
    if policy is not None:
        project["policy"] = str(absolute_path(policy, "policy"))
    projects = [existing for existing in config["projects"] if existing["repository"] != str(repository)]
    projects.append(project)
    if len(projects) > PROJECT_LIMIT:
        raise CaptureError("capture configuration supports at most 64 projects")
    rendered = json.dumps({"version": 1, "projects": projects}, ensure_ascii=False, indent=2).encode() + b"\n"
    if len(rendered) > CONFIG_LIMIT:
        raise CaptureError("capture configuration exceeds its size limit")
    if args.initialize_store:
        binary = Path(__file__).resolve().parent.parent / "skills/memento/bin/memento"
        command = [str(binary), "init", "--store", project["store"], "--project", project["project_id"]]
        if project.get("policy") is not None:
            command.extend(["--policy", project["policy"]])
        bounded_process(command,
                        b"", 10, "store initialization failed; configuration was not replaced")
    data.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=data, prefix=".capture-config-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(rendered)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.replace(destination)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    return {"configured": project, "config": str(destination), "hook_trust_modified": False}


def main(arguments=None):
    arguments = sys.argv[1:] if arguments is None else arguments
    try:
        if arguments:
            if arguments[0] != "configure":
                raise CaptureError("only the configure subcommand is supported")
            result = configure(arguments[1:])
        else:
            raw = sys.stdin.buffer.read(INPUT_LIMIT + 1)
            if len(raw) > INPUT_LIMIT:
                raise CaptureError("hook input exceeds 256 KiB; no complete event was captured")
            payload = json.loads(raw)
            if not isinstance(payload, dict):
                raise CaptureError("hook input must be an object")
            root = absolute_path(os.environ.get("PLUGIN_ROOT"), "PLUGIN_ROOT")
            data = absolute_path(os.environ.get("PLUGIN_DATA"), "PLUGIN_DATA")
            result = Dispatcher(payload, root, data).run()
    except (CaptureError, OSError, ValueError, TypeError, RecursionError, subprocess.SubprocessError) as error:
        # Never relay CLI stderr or raw input; backend redaction applies to saved details.
        result = {"systemMessage": f"memento capture_incomplete: {str(error)[:256]}"}
        code = 1 if arguments else 0
    else:
        code = 0
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
