import os


def create_script_from_embedded(ctx, filename=None, embedded=None):
    files_create_from_embedded_with_other_name = globals()[
        "files_create_from_embedded_with_other_name"
    ]

    # Create the file.
    files_create_from_embedded_with_other_name(
        ctx, filename_on_disk=filename, embedded_file=embedded
    )

    # Make the new file executable.
    os.chmod(filename, 0o755)
