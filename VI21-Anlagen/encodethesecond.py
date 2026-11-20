import subprocess
import time
from pathlib import Path


def output_path_for(file_name, output_dir):
    """Map an input below the current directory to a matching output path."""
    working_dir = Path.cwd().resolve()
    source = Path(file_name)
    if source.is_absolute():
        source = source.resolve().relative_to(working_dir)
    elif ".." in source.parts:
        raise ValueError("Input paths must stay inside the current directory")

    return Path(output_dir) / source.with_suffix(".mov")


def colliding_outputs(files, output_dir):
    outputs = {}
    collisions = set()
    for file_name in files:
        output_file = output_path_for(file_name, output_dir)
        if output_file in outputs:
            collisions.add(output_file)
        else:
            outputs[output_file] = file_name
    return collisions


def encode_videos():
    output_dir = Path("encoded")

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
    try:
        collisions = colliding_outputs(files, output_dir)
    except ValueError as error:
        print(f"❌ Error: {error}.")
        return

    for file_name in files:
        source = Path(file_name)
        if not source.is_file():
            print(f"❌ Error: File '{file_name}' not found in this folder. Skipping.")
            continue

        try:
            output_file = output_path_for(file_name, output_dir)
        except ValueError as error:
            print(f"❌ Error: {error}. Skipping '{file_name}'.")
            continue

        if output_file in collisions:
            print(f"❌ Error: Multiple inputs map to '{output_file}'. Skipping '{file_name}' to avoid overwriting.")
            continue
        if output_file.exists():
            print(f"❌ Error: Output '{output_file}' already exists. Skipping to avoid overwriting.")
            continue

        output_file.parent.mkdir(parents=True, exist_ok=True)

        print(f"\n🎬 Encoding: {file_name} -> {output_file}...")

        # Start timer for individual file
        file_start_time = time.time()

        # ffmpeg command built as a list for stability
        cmd = [
            "ffmpeg", "-n", "-i", file_name,
            "-c:v", "prores_ks", "-profile:v", "1",
            "-c:a", "pcm_s16le", output_file
        ]

        try:
            # Runs the command and shows progress in the terminal
            subprocess.run(cmd, check=True)

            # Calculate individual file elapsed time
            file_elapsed = time.time() - file_start_time
            print(f"✅ Successfully encoded: {output_file} (Took {file_elapsed:.2f}s)")

            # Add to our list of successful encodes
            successful_files.append(output_file)

        except subprocess.CalledProcessError:
            print(f"❌ Error: ffmpeg failed to encode '{file_name}'.")
        except FileNotFoundError:
            print("❌ Error: 'ffmpeg' command not found. Please install ffmpeg on your system.")
            return

    # --- FINAL SUMMARY SECTION ---
    total_elapsed = time.time() - total_start_time
    print("\n" + "="*40)
    print("🏁 ENCODING JOB COMPLETED")
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
