#!/usr/bin/env python3
"""
演示Python脚本，用于测试进度监控功能
这个脚本会模拟一个耗时任务，并输出进度信息
"""

import time
import sys
import json

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
        sys.stdout.flush()
        
        # 模拟一些可能的错误情况
        if i == 10:
            print("中途检查点：任务进行顺利")
            sys.stdout.flush()
    
    print("任务执行完成！")
    sys.stdout.flush()

def simulate_data_processing():
    """模拟数据处理任务"""
    data_items = [
        "处理用户数据",
        "验证数据格式", 
        "清理无效数据",
        "计算统计信息",
        "生成报告",
        "保存结果到数据库",
        "发送通知邮件",
        "清理临时文件",
        "更新缓存",
        "完成任务"
    ]
    
    print("开始数据处理任务...")
    sys.stdout.flush()
    
    for i, item in enumerate(data_items, 1):
        time.sleep(0.8)
        
        percentage = (i / len(data_items)) * 100
        print(f"进度: {percentage:.1f}% - {item}")
        sys.stdout.flush()
    
    print("数据处理完成！")
    sys.stdout.flush()

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "data":
        simulate_data_processing()
    else:
        simulate_long_task()
