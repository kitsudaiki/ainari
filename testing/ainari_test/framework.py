# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
Core of the test-framework.

A suite is a group of tests, which run in the order of their definition:

    suite = Suite("snapshots", "save and restore snapshots", depends=("ssh",))

    @suite.test("save snapshot", requires=("virtual_machines",), provides=("snapshot",))
    def save_snapshot(ctx):
        ...
        ctx.state["snapshot"] = snapshot_uuid

The tests share their resources over `ctx.state`. A test, whose required state-entries are missing,
because the test, which should provide them, failed or was not selected, is skipped instead of
failing with follow-up errors. Every created resource is registered in `ctx.cleanup`, so it is
deleted at the end, also when the run was aborted.
"""

import time
import traceback
import xml.etree.ElementTree as ElementTree
from dataclasses import dataclass, field

from ainari_sdk import ainari_exceptions

from .checks import CheckFailed, SkipTest
from .config import Config
from .ssh import Ssh

PASS = "PASS"
FAIL = "FAIL"
ERROR = "ERROR"
SKIP = "SKIP"

_COLORS = {PASS: "\033[32m", FAIL: "\033[31m", ERROR: "\033[31m", SKIP: "\033[33m"}
_RESET = "\033[0m"


@dataclass
class Test:
    name: str
    function: object
    requires: tuple = ()
    provides: tuple = ()


class Suite:
    def __init__(self, name: str, description: str, depends: tuple = (), always: bool = False):
        """
        depends: suites, which provide the resources of this suite. They are selected
                 automatically, if this suite is selected.
        always:  run this suite also after an abort by --fail-fast (used for the cleanup)
        """
        self.name = name
        self.description = description
        self.depends = depends
        self.always = always
        self.tests = []

    def test(self, name: str, requires: tuple = (), provides: tuple = ()):
        def decorator(function):
            self.tests.append(Test(name, function, tuple(requires), tuple(provides)))
            return function
        return decorator


@dataclass
class CleanupEntry:
    kind: str
    uuid: str
    label: str
    delete: object
    exists: object = None


class Cleanup:
    """
    Registry of all resources, which were created by the tests and not deleted yet.
    """

    def __init__(self, log):
        self._entries = []
        self._log = log

    def add(self, kind: str, uuid: str, label: str, delete, exists=None):
        self._entries.append(CleanupEntry(kind, uuid, label, delete, exists))

    def discard(self, uuid: str):
        self._entries = [entry for entry in self._entries if entry.uuid != uuid]

    def pending(self, kind: str = None) -> list:
        return [entry for entry in self._entries if kind is None or entry.kind == kind]

    def run(self, kind: str = None) -> list:
        """
        Deletes the registered resources in reverse order of their creation and returns the
        errors. A resource, which is already gone, counts as deleted.
        """
        errors = []
        for entry in reversed(self.pending(kind)):
            try:
                entry.delete()
                self._log(f"deleted {entry.kind} {entry.label} ({entry.uuid})")
            except ainari_exceptions.NotFoundException:
                self._log(f"{entry.kind} {entry.label} ({entry.uuid}) was already gone")
            except Exception as error:  # noqa: BLE001
                errors.append(f"{entry.kind} {entry.label} ({entry.uuid}): "
                              f"{type(error).__name__}: {error}")
                continue
            self.discard(entry.uuid)
        return errors


class Context:
    """
    Everything, which is shared between the tests of a run.
    """

    def __init__(self, config: Config, test_id: str):
        self.config = config
        self.test_id = test_id
        self.api = None  # access-context of the sdk, set by the runner after the login
        self.state = {}
        self.ssh = Ssh(config, self.log)
        self.cleanup = Cleanup(self.log)

    def name(self, suffix: str = "") -> str:
        """
        Unique name for a resource of this run.
        """
        base = f"local-stack-test-{self.test_id}"
        return f"{base}-{suffix}" if suffix else base

    @staticmethod
    def log(message: str):
        print(f"        {message}", flush=True)


@dataclass
class Result:
    suite: str
    test: str
    status: str
    duration: float = 0.0
    message: str = ""
    details: str = field(default="", repr=False)


class Runner:
    def __init__(self, suites: list, fail_fast: bool = False, keep: bool = False,
                 use_color: bool = True):
        self.suites = suites
        self.fail_fast = fail_fast
        self.keep = keep
        self.use_color = use_color
        self.results = []

    def _status(self, status: str) -> str:
        if not self.use_color:
            return status
        return f"{_COLORS[status]}{status}{_RESET}"

    def run(self, ctx: Context) -> list:
        aborted = False
        for suite in self.suites:
            if suite.always and self.keep:
                continue
            if aborted and not suite.always:
                for test in suite.tests:
                    self.results.append(Result(suite.name, test.name, SKIP, message="aborted"))
                continue

            print(f"\n=== {suite.name}: {suite.description}", flush=True)
            for test in suite.tests:
                if aborted and not suite.always:
                    self.results.append(Result(suite.name, test.name, SKIP, message="aborted"))
                    continue
                result = self._run_test(ctx, suite, test)
                self.results.append(result)
                if result.status in (FAIL, ERROR) and self.fail_fast:
                    aborted = True

        if not self.keep:
            # everything, which was not deleted by the cleanup-suite, is removed here
            leftovers = ctx.cleanup.pending()
            if leftovers:
                print(f"\n=== removing {len(leftovers)} leftover resource(s)", flush=True)
                for error in ctx.cleanup.run():
                    print(f"        cleanup failed: {error}", flush=True)
        return self.results

    def _run_test(self, ctx: Context, suite: Suite, test: Test) -> Result:
        missing = [key for key in test.requires if key not in ctx.state]
        if missing:
            result = Result(suite.name, test.name, SKIP, message=f"requires {', '.join(missing)}")
            print(f"  [{self._status(SKIP)}] {test.name} ({result.message})", flush=True)
            return result

        print(f"  [ RUN] {test.name}", flush=True)
        start = time.time()
        try:
            test.function(ctx)
            status, message, details = PASS, "", ""
        except SkipTest as error:
            status, message, details = SKIP, str(error), ""
        except CheckFailed as error:
            status, message, details = FAIL, str(error), traceback.format_exc()
        except Exception as error:  # noqa: BLE001
            status = ERROR
            message = f"{type(error).__name__}: {error}"
            details = traceback.format_exc()
        duration = time.time() - start

        if status == PASS:
            not_provided = [key for key in test.provides if key not in ctx.state]
            if not_provided:
                status = ERROR
                message = f"test did not provide {', '.join(not_provided)}"

        print(f"  [{self._status(status)}] {test.name} ({duration:.1f}s)", flush=True)
        if message:
            print(f"        -> {message}", flush=True)
        return Result(suite.name, test.name, status, duration, message, details)

    def print_summary(self):
        counts = {status: 0 for status in (PASS, FAIL, ERROR, SKIP)}
        for result in self.results:
            counts[result.status] += 1

        print("\n=== summary", flush=True)
        for result in self.results:
            if result.status in (FAIL, ERROR):
                print(f"  [{self._status(result.status)}] {result.suite} / {result.test}: "
                      f"{result.message}")
        total_time = sum(result.duration for result in self.results)
        print(f"  {counts[PASS]} passed, {counts[FAIL]} failed, {counts[ERROR]} errors, "
              f"{counts[SKIP]} skipped in {total_time:.0f}s", flush=True)

    def successful(self) -> bool:
        return all(result.status in (PASS, SKIP) for result in self.results)

    def write_junit(self, path: str):
        testsuites = ElementTree.Element("testsuites")
        for suite in self.suites:
            results = [result for result in self.results if result.suite == suite.name]
            if not results:
                continue
            element = ElementTree.SubElement(
                testsuites, "testsuite",
                name=suite.name,
                tests=str(len(results)),
                failures=str(sum(result.status == FAIL for result in results)),
                errors=str(sum(result.status == ERROR for result in results)),
                skipped=str(sum(result.status == SKIP for result in results)),
                time=f"{sum(result.duration for result in results):.3f}")
            for result in results:
                case = ElementTree.SubElement(element, "testcase", classname=suite.name,
                                              name=result.test, time=f"{result.duration:.3f}")
                if result.status == FAIL:
                    ElementTree.SubElement(case, "failure",
                                           message=result.message).text = result.details
                elif result.status == ERROR:
                    ElementTree.SubElement(case, "error",
                                           message=result.message).text = result.details
                elif result.status == SKIP:
                    ElementTree.SubElement(case, "skipped", message=result.message)
        ElementTree.ElementTree(testsuites).write(path, encoding="utf-8", xml_declaration=True)


def select_suites(all_suites: list, only: list, skip: list) -> list:
    """
    Returns the suites to run in their defined order. Selected suites pull in the suites, which
    they depend on, and suites, which always run, like the cleanup, are always part of it.
    """
    by_name = {suite.name: suite for suite in all_suites}
    for name in list(only) + list(skip):
        if name not in by_name:
            raise ValueError(f"unknown suite '{name}', available: {', '.join(by_name)}")

    if only:
        selected = set()
        pending = list(only)
        while pending:
            name = pending.pop()
            if name not in selected:
                selected.add(name)
                pending.extend(by_name[name].depends)
    else:
        selected = set(by_name)
    selected -= set(skip)

    return [suite for suite in all_suites if suite.name in selected or suite.always]
