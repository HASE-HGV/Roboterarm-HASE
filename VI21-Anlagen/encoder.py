import subprocess
import time  # Imported to track elapsed time
from pathlib import Path

def encode_videos():
    # Ensure the output directory exists
    output_dir = Path("encoded")
    if not output_dir.exists():
        output_dir.mkdir(parents=True)
        print(f"Created directory: {output_dir}/")

    print("--- DaVinci Resolve Linux Video Converter ---")
    print("Enter the filenames you want to convert (e.g., C0045.mp4).")
    print("Separate multiple files with a comma. Type 'exit' to quit.\n")

    user_input = input("Enter filename(s): ")
    if user_input.strip().lower() == 'exit':
        return

    # Split inputs by comma and strip extra whitespaces
    files = [f.strip() for f in user_input.split(",") if f.strip()]

    # Trackers for the final summary
    successful_files = []
    total_start_time = time.time()

    for file_name in files:
        source_path = Path(file_name).expanduser()
        if not source_path.is_file():
            print(f"Error: Input '{file_name}' is not an existing file. Skipping.")
            continue

        # Keep relative subfolders under encoded/, while absolute inputs use
        # only their filename so they cannot escape the output directory.
        if source_path.is_absolute():
            relative_source = Path(source_path.name)
        else:
            relative_source = source_path
            if ".." in relative_source.parts:
                print(f"Error: Input path '{file_name}' must not contain '..'. Skipping.")
                continue

        output_file = output_dir / relative_source.with_suffix(".mov")
        if output_file.exists():
            print(f"Error: Output '{output_file}' already exists; refusing to overwrite. Skipping.")
            continue

        try:
            output_file.parent.mkdir(parents=True, exist_ok=True)
        except OSError as error:
            print(f"Error: Could not create output directory '{output_file.parent}': {error}")
            continue

        print(f"\nEncoding: {file_name} -> {output_file}...")

        # Start timer for individual file
        file_start_time = time.time()

        # ffmpeg command built as a list for stability
        cmd = [
            "ffmpeg", "-n", "-i", str(source_path),
            "-c:v", "prores_ks", "-profile:v", "1",
            "-c:a", "pcm_s16le", str(output_file)
        ]

        try:
            # Runs the command and shows progress in the terminal
            subprocess.run(cmd, check=True)

            # Calculate individual file elapsed time
            file_elapsed = time.time() - file_start_time
            print(f"Successfully encoded: {output_file} (Took {file_elapsed:.2f}s)")

            # Add to our list of successful encodes
            successful_files.append(output_file)

        except subprocess.CalledProcessError as error:
            print(f"Error: ffmpeg failed to encode '{file_name}' (exit status {error.returncode}).")
        except FileNotFoundError:
            print("Error: 'ffmpeg' command not found. Please install ffmpeg on your system.")
            return

    # --- FINAL SUMMARY SECTION ---
    total_elapsed = time.time() - total_start_time
    print("\n" + "="*40)
    print("ENCODING JOB COMPLETED")
    print(f"Total time elapsed: {total_elapsed:.2f} seconds")

    if successful_files:
        print(f"\nSuccessfully encoded {len(successful_files)} file(s):")
        for f in successful_files:
            print(f" - {f}")
    else:
        print("\nNo files were successfully encoded.")
    print("="*40)

if __name__ == "__main__":
    encode_videos()
