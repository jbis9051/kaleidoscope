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


# sometimes whisper produces bad output, we do a few checks
# this is based on advice from umich ITS lecture capture department
def valid_transcript(segments):
    words = []
    for segment in segments:
        words.append(segment.text.strip())
    words = ' '.join(words).lower().split(' ')

    # get rid of short transcripts
    if len(words) <= 3:
        return False
    # check for 1,2,3 repeated words
    for i in range(1, 4):
        beg = '|'.join(words[0:i])
        repeats = 1
        for j in range(i+1, len(words), i):
            sec = words[j:(j+i)]
            sec = '|'.join(sec)
            if beg == sec:
                repeats += 1
            else:
                repeats = 0
            if repeats > 1:
                return False
    return True

segments = list(segments)
if not valid_transcript(segments):
    print("failure")
    exit(0)

print("success")
print('%s' % info.language)
print('%f' % info.language_probability)

for segment in segments:
    print("%f|%f|%s" % (segment.start, segment.end, segment.text.strip()))