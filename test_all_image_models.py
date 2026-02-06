#!/usr/bin/env python3
"""
测试 LaoZhang 所有图像生成模型

运行方式:
python test_all_image_models.py
"""

import os
import requests
import json
from typing import Optional, Dict, Any

# API 配置
API_KEY = os.getenv("LAOZHANG_API_KEY", "sk-6KvXE35B3uK9fXDE49Cb622b72264b6dAeA44a9fA3F8B979")
BASE_URL = "https://api.laozhang.ai"

# 测试配置
TEST_PROMPT = "A serene Japanese garden with cherry blossoms in spring"

# 所有要测试的模型
MODELS = [
    {
        "name": "gpt-4o-image",
        "size": "1024x1024",
        "description": "GPT-4o Image - 推荐，性价比高"
    },
    {
        "name": "dall-e-3",
        "size": "1024x1024",
        "quality": "hd",
        "style": "vivid",
        "description": "DALL-E 3 - 理解能力强"
    },
    {
        "name": "dall-e-2",
        "size": "512x512",
        "description": "DALL-E 2 - 经典版本"
    },
    {
        "name": "black-forest-labs/flux-pro-v1.1",
        "size": "1024x1024",
        "description": "Flux Pro - 专业质量"
    },
    {
        "name": "claude-3.5-sonnet-img",
        "size": "1024x1024",
        "description": "Claude 3.5 Sonnet Image"
    },
]


def generate_image(model_config: Dict[str, Any]) -> Optional[str]:
    """生成图像并返回 URL"""
    url = f"{BASE_URL}/v1/images/generations"
    
    # 构建请求
    payload = {
        "model": model_config["name"],
        "prompt": TEST_PROMPT,
        "n": 1,
        "size": model_config.get("size", "1024x1024")
    }
    
    # 添加可选参数
    if "quality" in model_config:
        payload["quality"] = model_config["quality"]
    if "style" in model_config:
        payload["style"] = model_config["style"]
    
    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json"
    }
    
    try:
        print(f"\n📤 请求模型: {model_config['name']}")
        print(f"   描述: {model_config['description']}")
        print(f"   尺寸: {payload['size']}")
        print(f"   提示词: {TEST_PROMPT}")
        
        response = requests.post(url, headers=headers, json=payload, timeout=60)
        response_data = response.json()
        
        if response.status_code == 200:
            if "data" in response_data and len(response_data["data"]) > 0:
                image_url = response_data["data"][0]["url"]
                print(f"✅ 成功! 图片 URL: {image_url[:80]}...")
                return image_url
            else:
                print(f"❌ 响应格式错误: {response_data}")
                return None
        else:
            print(f"❌ 失败! 状态码: {response.status_code}")
            print(f"   错误信息: {json.dumps(response_data, indent=2, ensure_ascii=False)}")
            return None
            
    except requests.exceptions.Timeout:
        print(f"⏱️  超时! 请求超过 60 秒")
        return None
    except Exception as e:
        print(f"❌ 异常: {str(e)}")
        return None


def main():
    """主测试函数"""
    print("=" * 80)
    print(" LaoZhang 图像生成模型测试")
    print("=" * 80)
    print(f"\n🔑 API Key: {API_KEY[:20]}...")
    print(f"🌐 Base URL: {BASE_URL}")
    print(f"📝 测试提示词: {TEST_PROMPT}")
    print(f"🧪 测试模型数量: {len(MODELS)}")
    
    results = []
    
    for i, model_config in enumerate(MODELS, 1):
        print(f"\n{'─' * 80}")
        print(f"测试 {i}/{len(MODELS)}")
        
        image_url = generate_image(model_config)
        
        results.append({
            "model": model_config["name"],
            "description": model_config["description"],
            "success": image_url is not None,
            "url": image_url
        })
    
    # 输出总结
    print(f"\n{'=' * 80}")
    print(" 测试总结")
    print("=" * 80)
    
    success_count = sum(1 for r in results if r["success"])
    total_count = len(results)
    
    print(f"\n✅ 成功: {success_count}/{total_count}")
    print(f"❌ 失败: {total_count - success_count}/{total_count}")
    print(f"📊 成功率: {success_count / total_count * 100:.1f}%")
    
    print(f"\n{'─' * 80}")
    print("详细结果:")
    print("─" * 80)
    
    for result in results:
        status = "✅" if result["success"] else "❌"
        print(f"{status} {result['model']:40} - {result['description']}")
        if result["success"] and result["url"]:
            print(f"   URL: {result['url'][:80]}...")
    
    print("\n" + "=" * 80)
    
    # 保存结果到文件
    output_file = "image_generation_test_results.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump({
            "total": total_count,
            "success": success_count,
            "failure": total_count - success_count,
            "success_rate": f"{success_count / total_count * 100:.1f}%",
            "results": results
        }, f, indent=2, ensure_ascii=False)
    
    print(f"\n💾 结果已保存到: {output_file}")
    
    # 返回退出码
    return 0 if success_count == total_count else 1


if __name__ == "__main__":
    exit(main())
