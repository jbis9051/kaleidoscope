from faster_whisper import WhisperModel
import sys

if len(sys.argv) != 6:
    print("Usage: python fw-transcribe.py <model> <device> <compute_type> <download_root> <audio_file>")
    sys.exit(1)

model_size = sys.argv[1]
device = sys.argv[2]
compute_type = sys.argv[3]
download_root = sys.argv[4]
audio_file = sys.argv[5]

model = WhisperModel(model_size, device=device, compute_type=compute_type, download_root=download_root)
segments, info = model.transcribe(audio_file, beam_size=5, language="en", condition_on_previous_text=False, vad_filter=False)


print('%s' % info.language)
print('%f' % info.language_probability)

for segment in segments:
    print("%f|%f|%s" % (segment.start, segment.end, segment.text.strip()))