import os
import subprocess
import time  # Imported to track elapsed time

def encode_videos():
    # Ensure the output directory exists
    output_dir = "encoded"
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
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
        if not os.path.exists(file_name):
            print(f"❌ Error: File '{file_name}' not found in this folder. Skipping.")
            continue

        # Extract filename without extension to name the output file
        base_name, _ = os.path.splitext(file_name)
        output_file = os.path.join(output_dir, f"{base_name}.mov")

        print(f"\n🎬 Encoding: {file_name} -> {output_file}...")

        # Start timer for individual file
        file_start_time = time.time()

        # ffmpeg command built as a list for stability
        cmd = [
            "ffmpeg", "-y", "-i", file_name,
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
            print(f"❌ Error: Failed to encode '{file_name}'. Is ffmpeg installed?")
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
