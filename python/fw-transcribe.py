from faster_whisper import WhisperModel
import sys
import re

if len(sys.argv) != 6:
    print("Usage: python fw-transcribe.py <model> <device> <compute_type> <download_root> <audio_file>")
    sys.exit(1)

model_size = sys.argv[1]
device = sys.argv[2]
compute_type = sys.argv[3]
download_root = sys.argv[4]
audio_file = sys.argv[5]

# supposedly overriding the default supress_tokens reduces hallucinations https://github.com/linto-ai/whisper-timestamped/discussions/107

model = WhisperModel(model_size, device=device, compute_type=compute_type, download_root=download_root)
segments, info = model.transcribe(audio_file, beam_size=5, language="en", condition_on_previous_text=False,
                                  vad_filter=False, suppress_tokens=None)


# sometimes whisper produces bad output, we do a few checks
# this is based on advice from umich ITS lecture capture department
#
# this is written pretty horrendously but i'm lazy
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
        for j in range(i + 1, len(words), i):
            sec = words[j:(j + i)]
            sec = '|'.join(sec)
            if beg == sec:
                repeats += 1
            else:
                repeats = 0
            if repeats > 1:
                return False
    common_bad = ["Thank you for watching!",
                  "Thank you very much!",
                  "We'll be right back",
                  "Thank you for watching, please subscribe, like, comment, and share this video!",
                  "Subscribe to MrFudgeMonkeyz for more funny minecraft animations!"
                  "Thank you so much for watching, and please subscribe to my channel!",
                  "I invite you guys to subscribe"
                  "Thank you for watching, please subscribe, like, comment and share this video."
                  "This is the end of the video. If you like this video, please subscribe to the channel and give it a thumbs up! Thank you for watching and see you in the next video!"
                  ]
    if len(words) > 30:
        words_concat = ' '.join(re.findall(r'[a-zA-Z0-9]', ' '.join(words)))
        for bad in common_bad:
            bad = bad.lower()
            bad = bad.split(' ')
            bad = bad[0:min(len(bad), 6)]
            bad = ' '.join(bad)
            bad = ' '.join(re.findall(r'[a-zA-Z0-9]', bad))
            if words_concat.startswith(bad):
                return False
        begin = words[0:min(len(words), 10)]
        matches = 0
        for bad in ["thank", "thanks", "subscribe", "watching", "invite"]:
            if bad in begin:
                matches += 1
        if matches >= 2:
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
