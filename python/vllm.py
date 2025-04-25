import json
import sys
import time
import os
os.environ["TRANSFORMERS_NO_TQDM"] = "1"
os.environ["HF_HUB_DISABLE_PROGRESS_BARS"] = "1"

REVISION = 'e474b39487e563dbfa12c12e6a7e7d743cf340d4'


from pathlib import Path

import huggingface_hub
snapshot_download = huggingface_hub.snapshot_download

def snapshot_download_safe(*args, **kwargs):
    if not kwargs['revision']:
        print("snapshot_download was called without revision: ", *args, kwargs)
        exit(1)
    return snapshot_download(*args, **kwargs)

huggingface_hub.snapshot_download = snapshot_download_safe

import mlx_vlm

get_model_path = mlx_vlm.utils.get_model_path

def get_model_path_safe(path_or_hf_repo):
    model_path = Path(path_or_hf_repo)
    if not model_path.exists():
        model_path = Path(
            snapshot_download_safe(
                repo_id=path_or_hf_repo,
                revision=REVISION,
                allow_patterns=[
                    "*.json",
                    "*.safetensors",
                    "*.py",
                    "tokenizer.model",
                    "*.tiktoken",
                    "*.txt",
                ],
            )
        )
    return model_path

mlx_vlm.utils.get_model_path = get_model_path_safe

########################################################################################################################

"""
Output Format:

<float> <-- load time
----- below repeated runs times
<int> <--- iteration
<float> <--- iteration time
<str> <-- JSON output string 
"""


from mlx_vlm import apply_chat_template, load
from mlx_vlm.utils import load_config
from mlx_vlm import load, generate

if len(sys.argv) != 5:
    print("Usage: python vllm <prompt> <image_path> <max_tokens> <runs>")
    sys.exit(1)

prompt = sys.argv[1]
image_path = sys.argv[2]
max_tokens = int(sys.argv[3])
runs = int(sys.argv[4])

model_path = "mlx-community/InternVL3-2B-4bit"

load_start = time.time()
model, processor = load(model_path, trust_remote_code=False, local_files_only=True, revision="foo")
config = load_config(model_path, trust_remote_code=False, local_files_only=True)

# load
print(time.time()-load_start)

# Prepare input
image = [image_path]

formatted_prompt = apply_chat_template(
    processor, config, prompt, num_images=len(image)
)


for i in range(runs):
    # Generate output
    start = time.time()
    output = generate(model, processor, formatted_prompt, image, verbose=False, temperature=0.5, max_tokens=max_tokens)
    end = time.time()
    print(i)
    print(end-start)
    print(json.dumps(output))