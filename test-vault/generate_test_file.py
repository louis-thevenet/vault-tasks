#!/usr/bin/env python3

from pathlib import Path
import argparse
import random


WORDS = [
    "lorem",
    "ipsum",
    "blablabla",
]


def make_header():
    level = random.choice([1, 2, 3, 4])
    title = " ".join(random.choices(WORDS, k=random.randint(2, 5)))
    return f"{'#' * level} {title}\n\n"


def make_task():
    status = random.choice([" ", "x", "-", "/"])
    completion = random.randint(0, 100)
    priority = random.randint(0, 5)

    tags = " ".join(
        f"#{random.choice(['tag1', 'tag2', 'work', 'home'])}"
        for _ in range(random.randint(0, 3))
    )

    task = f"- [{status}] task c{completion} p{priority} {tags}".rstrip()
    task += "\n"

    if random.random() < 0.7:
        desc = random.choice(
            [
                "simple description",
                "_markdown_ **styled** text",
                "description with `code`",
                "quoted text > hello",
            ]
        )
        task += f"  {desc}\n"

    return task + "\n"


def make_text():
    return " ".join(random.choices(WORDS, k=random.randint(5, 20))) + "\n\n"


def pick_block(weights):
    r = random.random()
    acc = 0.0

    for kind, weight in weights.items():
        acc += weight
        if r <= acc:
            return kind

    return "text"


def generate_file(
    output_path: Path,
    target_size_kb: int,
    header_ratio: float,
    task_ratio: float,
    text_ratio: float,
):
    weights = {
        "header": header_ratio,
        "task": task_ratio,
        "text": text_ratio,
    }

    total = sum(weights.values())
    weights = {k: v / total for k, v in weights.items()}

    generators = {
        "header": make_header,
        "task": make_task,
        "text": make_text,
    }

    target_size = target_size_kb * 1024

    blocks = []
    current_size = 0

    header_count = 0
    task_count = 0
    plain_line_count = 0

    while current_size < target_size:
        kind = pick_block(weights)
        block = generators[kind]()

        blocks.append(block)
        current_size += len(block.encode("utf-8"))

        if kind == "header":
            header_count += 1

        elif kind == "task":
            task_count += 1

        elif kind == "text":
            plain_line_count += sum(1 for line in block.splitlines() if line.strip())

    content = "".join(blocks)

    content = content.encode("utf-8")[:target_size].decode("utf-8", errors="ignore")

    content += (
        "\n\n---\n"
        f"Headers: {header_count}\n"
        f"Tasks: {task_count}\n"
        f"Plain lines: {plain_line_count}\n"
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(content, encoding="utf-8")

    print(f"Generated: {output_path}")
    print(f"Actual size: {output_path.stat().st_size} bytes")
    print(f"Headers: {header_count}")
    print(f"Tasks: {task_count}")
    print(f"Plain lines: {plain_line_count}")


def main():
    parser = argparse.ArgumentParser(description="Generate random markdown corpus")

    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("./test-vault/generated.md"),
        help="Output markdown file",
    )

    parser.add_argument(
        "-s",
        "--size-kb",
        type=int,
        default=180,
        help="Target size in KiB",
    )

    parser.add_argument(
        "--headers",
        type=float,
        default=0.15,
        help="Header ratio",
    )

    parser.add_argument(
        "--tasks",
        type=float,
        default=0.65,
        help="Task ratio",
    )

    parser.add_argument(
        "--text",
        type=float,
        default=0.20,
        help="Plain text ratio",
    )

    args = parser.parse_args()

    generate_file(
        output_path=args.output,
        target_size_kb=args.size_kb,
        header_ratio=args.headers,
        task_ratio=args.tasks,
        text_ratio=args.text,
    )


if __name__ == "__main__":
    main()
