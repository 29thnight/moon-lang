import requests
import json
import re

def fetch_toc(version="6000.4"):
    url = f"https://docs.unity3d.com/{version}/Documentation/ScriptReference/docdata/toc.js"
    response = requests.get(url)
    if response.status_code != 200:
        raise Exception(f"Failed to fetch TOC for {version}: {response.status_code}")
    
    text = response.text.strip()
    # 'var toc = {...};' 형식을 정규식으로 추출
    match = re.match(r'^var\s+toc\s*=\s*(.*);?$', text, re.DOTALL)
    if not match:
        try:
            return json.loads(text)
        except:
            raise Exception(f"Could not parse toc.js for {version}")
            
    json_str = match.group(1).rstrip(';')
    try:
        return json.loads(json_str)
    except json.JSONDecodeError as e:
        print(f"JSON Decode Error for {version}: {e}")
        raise e

def extract_items(node, items=None):
    if items is None:
        items = []
    
    if not isinstance(node, dict):
        return items
        
    title = node.get('title', '')
    link = node.get('link', '')
    
    # 실제 페이지 링크가 있는 항목만 추출 (필터링은 main.py에서 수행)
    if link and link != 'toc' and link != 'index' and link != 'null':
        items.append({
            'name': title,
            'url_path': link
        })
        
    children = node.get('children', [])
    if isinstance(children, list):
        for child in children:
            extract_items(child, items)
            
    return items

if __name__ == "__main__":
    print("Testing TOC for 6000.0...")
    toc = fetch_toc("6000.0")
    print(f"Name: {toc.get('title')}, Children: {len(toc.get('children', []))}")
