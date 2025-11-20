import logging
import os
import tempfile


#############################################################################
# Code to implement the scenarios.


class Step:
    def __init__(self):
        self._kind = None
        self._text = None
        self._args = {}
        self._function = None
        self._cleanup = None

    def set_kind(self, kind):
        self._kind = kind

    def set_text(self, text):
        self._text = text

    def set_arg(self, name, value):
        self._args[name] = value

    def set_function(self, function):
        self._function = function

    def set_cleanup(self, cleanup):
        self._cleanup = cleanup

    def do(self, ctx):
        print("  step: {} {}".format(self._kind, self._text))
        logging.info("step: {} {}".format(self._kind, self._text))
        self._function(ctx, **self._args)

    def cleanup(self, ctx):
        if self._cleanup:
            print("  cleanup: {} {}".format(self._kind, self._text))
            logging.info("cleanup: {} {}".format(self._kind, self._text))
            self._cleanup(ctx, **self._args)


_logged_env = False


class Scenario:
    def __init__(self, ctx):
        self._title = None
        self._steps = []
        self._ctx = ctx

    def get_title(self):
        return self._title

    def set_title(self, title):
        self._title = title

    def append_step(self, step):
        self._steps.append(step)

    def run(self, datadir, extra_env):
        print("scenario: {}".format(self._title))
        logging.info("Scenario: {}".format(self._title))

        scendir = tempfile.mkdtemp(dir=datadir)
        os.chdir(scendir)
        self._set_environment_variables_to(scendir, extra_env)

        done = []
        ctx = self._ctx
        try:
            for step in self._steps:
                step.do(ctx)
                done.append(step)
        except Exception as e:
            logging.error(str(e), exc_info=True)
            for step in reversed(done):
                step.cleanup(ctx)
            raise
        for step in reversed(done):
            step.cleanup(ctx)

    def _set_environment_variables_to(self, scendir, extra_env):
        log_value = globals()["log_value"]

        overrides = {
            "SHELL": "/bin/sh",
            "HOME": scendir,
            "TMPDIR": scendir,
        }

        os.environ.update(overrides)
        os.environ.update(extra_env)
        global _logged_env
        if not _logged_env:
            _logged_env = True
            log_value("extra_env", 0, dict(extra_env))
            log_value("overrides", 0, dict(overrides))
            log_value("os.environ", 0, dict(os.environ))
