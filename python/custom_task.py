import inspect
import json
import sys

def _call_fn(has_return: bool, *args, **kwargs):
    name = inspect.currentframe().f_back.f_code.co_name

    out = {
        "name": name,
        "args": args,
        "kwargs": kwargs
    }

    out_str = json.dumps(out)
    print(out_str, flush=True)
    if has_return:
        line = sys.stdin.readline()
        return json.loads(line)

# dict["media"], dict["version"]
def parse_input():
    line = sys.stdin.readline()
    return json.loads(line)
def execute_task(task_name: str, *args):
    return _call_fn(True, task_name, json.dumps(args))
def add_tag(tag_name: str) -> bool:
    return _call_fn(True, tag_name)
def remove_tag(tag_name: str) -> bool:
    return _call_fn(True, tag_name)
def add_metadata(key: str, value: str) -> bool:
    return _call_fn(True, key, value)
def delete_metadata(key: str) -> bool:
    return _call_fn(True, key)
def get_metadata(key: str) -> str | None:
    return _call_fn(True, key)
def log(*args):
    return _call_fn(False, *args)