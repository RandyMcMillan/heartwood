def clone_to_src(ctx, url=None, commit=None):
    runcmd_run = globals()["runcmd_run"]
    runcmd_exit_code_is_zero = globals()["runcmd_exit_code_is_zero"]

    runcmd_run(ctx, ["git", "clone", url, "src"])
    runcmd_exit_code_is_zero(ctx)

    runcmd_run(ctx, ["git", "checkout", commit], cwd="src")
    runcmd_exit_code_is_zero(ctx)


def docgen(ctx, filename=None, output=None, dirname=None):
    runcmd_run = globals()["runcmd_run"]
    runcmd_exit_code_is_zero = globals()["runcmd_exit_code_is_zero"]

    runcmd_run(
        ctx,
        ["subplot", "docgen", filename, "--merciful", "--output", output, "-t", "python"],
        cwd=dirname,
    )
    runcmd_exit_code_is_zero(ctx)
