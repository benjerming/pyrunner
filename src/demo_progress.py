#!/usr/bin/env python3
"""
演示Python脚本，用于测试进度监控功能
这个脚本会模拟一个耗时任务，并输出进度信息
"""

import json
import sys
import time


def simulate_long_task():
    """模拟一个耗时任务"""
    total_steps = 10

    print("开始执行耗时任务...")
    sys.stdout.flush()

    for i in range(1, total_steps + 1):
        # 模拟工作
        time.sleep(0.1)

        # 输出进度信息
        percentage = (i / total_steps) * 100
        message = f"正在处理步骤 {i}/{total_steps}"

        print(f"进度: {percentage:.1f}% - {message}")
        # {"Progress":{"done":1,"size":10}}
        print(json.dumps({"Progress": {"done": i, "size": total_steps}}), flush=True)

        # 模拟一些可能的错误情况
        if i == 10:
            print("中途检查点：任务进行顺利")

    print("任务执行完成！")
    print(json.dumps({"Result": {"pages": 10, "words": 100}}), flush=True)


if __name__ == "__main__":
    simulate_long_task()
