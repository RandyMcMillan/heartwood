import argparse
import logging
import os
import random
import shutil
import sys
import tarfile
import tempfile


class MultilineFormatter(logging.Formatter):
    def format(self, record):
        s = super().format(record)
        lines = list(s.splitlines())
        return lines.pop(0) + "\n".join("    %s" % line for line in lines)


def indent(n):
    return "  " * n


def log_value(msg, level, v):
    if is_multiline_string(v):
        logging.debug(f"{indent(level)}{msg}:")
        log_lines(indent(level + 1), v)
    elif isinstance(v, dict) and v:
        # Only non-empty dictionaries
        logging.debug(f"{indent(level)}{msg}:")
        for k in sorted(v.keys()):
            log_value(f"{k!r}", level + 1, v[k])
    elif isinstance(v, list) and v:
        # Only non-empty lists
        logging.debug(f"{indent(level)}{msg}:")
        for i, x in enumerate(v):
            log_value(f"{i}", level + 1, x)
    else:
        logging.debug(f"{indent(level)}{msg}: {v!r}")


def is_multiline_string(v):
    if isinstance(v, str) and "\n" in v:
        return True
    elif isinstance(v, bytes) and b"\n" in v:
        return True
    else:
        return False


def log_lines(prefix, v):
    if isinstance(v, str):
        nl = "\n"
    else:
        nl = b"\n"
    if nl in v:
        for line in v.splitlines(keepends=True):
            logging.debug(f"{prefix}{line!r}")
    else:
        logging.debug(f"{prefix}{v!r}")


# Remember where we started from. The step functions may need to refer
# to files there.
srcdir = os.getcwd()
print("srcdir", srcdir)

# Create a new temporary directory and chdir there. This allows step
# functions to create new files in the current working directory
# without having to be so careful.
_datadir = tempfile.mkdtemp()
print("datadir", _datadir)
os.chdir(_datadir)


def parse_command_line():
    p = argparse.ArgumentParser()
    p.add_argument("--log")
    p.add_argument("--env", action="append", default=[])
    p.add_argument("--run-all", "-k", action="store_true")
    p.add_argument("--save-on-failure")
    p.add_argument("patterns", nargs="*")
    return p.parse_args()


def setup_logging(args):
    if args.log:
        fmt = "%(asctime)s %(levelname)s %(message)s"
        datefmt = "%Y-%m-%d %H:%M:%S"
        formatter = MultilineFormatter(fmt, datefmt)

        filename = os.path.abspath(os.path.join(srcdir, args.log))
        handler = logging.FileHandler(filename)
        handler.setFormatter(formatter)
    else:
        handler = logging.NullHandler()

    logger = logging.getLogger()
    logger.addHandler(handler)
    logger.setLevel(logging.DEBUG)


def save_directory(dirname, tarname):
    print("tarname", tarname)
    logging.info("Saving {} to {}".format(dirname, tarname))
    tar = tarfile.open(tarname, "w")
    tar.add(dirname, arcname="datadir")
    tar.close()


def main(scenarios):
    args = parse_command_line()
    setup_logging(args)
    logging.info("Test program starts")

    logging.info("patterns: {}".format(args.patterns))
    if len(args.patterns) == 0:
        logging.info("Executing all scenarios")
        todo = list(scenarios)
        random.shuffle(todo)
    else:
        logging.info("Executing requested scenarios only: {}".format(args.patterns))
        patterns = [arg.lower() for arg in args.patterns]
        todo = [
            scen
            for scen in scenarios
            if any(pattern in scen.get_title().lower() for pattern in patterns)
        ]

    extra_env = {}
    for env in args.env:
        (name, value) = env.split("=", 1)
        extra_env[name] = value

    errors = []
    for scen in todo:
        try:
            scen.run(_datadir, extra_env)
        except Exception as e:
            logging.error(str(e), exc_info=True)
            errors.append((scen, e))
            if args.save_on_failure:
                print(args.save_on_failure)
                filename = os.path.abspath(os.path.join(srcdir, args.save_on_failure))
                print(filename)
                save_directory(_datadir, filename)
            if not args.run_all:
                raise

    shutil.rmtree(_datadir)

    if errors:
        sys.stderr.write(f"ERROR: {len(errors)} scenarios failed\n")
        for (scean, e) in errors:
            sys.stderr.write(f" - Scenario {scen.get_title()} failed:\n  {e}\n")
        if args.log:
            sys.stderr.write(f"Log file in {args.log}\n")
        sys.exit(1)

    print("OK, all scenarios finished successfully")
    logging.info("OK, all scenarios finished successfully")
