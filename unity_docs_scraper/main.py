import time
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed
from toc_parser import fetch_toc
from db_manager import DBManager
from page_parser import PageParser

# --- 수집 설정 ---
VERSIONS = ["6000.0", "6000.1", "6000.2", "6000.3", "6000.4", "6000.5", "6000.6"]
MAX_THREADS = 8
RETRY_DELAY = 1
MAX_RETRIES = 3
SCRAPE_SIGNATURES = True

session = requests.Session()

def discover_nodes(node, db, version, parent_id=None):
    """TOC를 끝까지 탐색하며 실제 문서 페이지가 있는 모든 노드를 수집 (무조건 탐색)"""
    if not node: return
    
    title = node.get('title', '')
    link = node.get('link', '')
    is_real_page = link and link not in ["null", "toc", "index"]

    # 1. 노드 정보 DB 확인 및 삽입
    if is_real_page:
        # 실제 페이지인 경우
        existing = db.exists(link, version)
        if existing:
            symbol_id = existing[0]
        else:
            symbol_id = db.insert_symbol({
                'parent_id': parent_id,
                'name': title,
                'full_name': title,
                'kind': 'Symbol',  # 나중에 scrape_worker에서 Class/Struct 등으로 업데이트됨
                'url': link,
                'version': version,
                'is_scraped': 0
            })
    else:
        # 폴더(Namespace 등)인 경우
        existing = db.find_node(title, parent_id, version)
        if existing:
            symbol_id = existing[0]
        else:
            symbol_id = db.insert_symbol({
                'parent_id': parent_id,
                'name': title,
                'full_name': title,
                'kind': 'Namespace',
                'url': None,
                'version': version,
                'is_scraped': 1 # 폴더는 상세 수집 대상이 아님
            })

    # 2. 자식 탐색 (항상 끝까지 파고듭니다)
    children = node.get('children')
    if isinstance(children, list):
        for child in children:
            discover_nodes(child, db, version, symbol_id)

def scrape_worker(item, db, parser, version):
    """상세 수집: 페이지를 열어 실제 종류(Class/Struct 등)와 멤버, 시그니처 수집"""
    sid, name, url = item
    
    retries = 0
    soup = None
    while retries < MAX_RETRIES:
        try:
            time.sleep(0.1)
            soup = parser.fetch_soup(url, version=version, session=session)
            if soup: break
            retries += 1
            time.sleep(RETRY_DELAY * retries)
        except:
            retries += 1
            time.sleep(RETRY_DELAY)

    if not soup: return f"Fetch Failed: {name}"

    details = parser.parse_class_page(soup, url)
    if not details: 
        # 파싱 실패 시 기본 데이터라도 채움
        db.update_detail(sid)
        return f"Parse Failed: {name}"

    # 1. 클래스 정보(Kind, Summary) 업데이트
    # 여기서 kind를 정확히 Class, Struct, Enum 등으로 교체합니다.
    db.update_detail(sid, kind=details['category'], summary=details['summary'])
    
    # 2. 멤버 목록 수집
    member_count = 0
    for member in details['members']:
        m_signature = ""
        if SCRAPE_SIGNATURES and member['link']:
            m_soup = parser.fetch_soup(member['link'], version=version, session=session)
            if m_soup:
                m_signature = parser.parse_signature(m_soup)
        
        db.insert_symbol({
            'parent_id': sid,
            'name': member['name'],
            'full_name': f"{name}.{member['name']}",
            'kind': member['member_type'],
            'summary': member['summary'],
            'signature': m_signature,
            'url': member['link'],
            'version': version,
            'is_scraped': 1
        })
        member_count += 1
    
    db.update_detail(sid) # 수집 완료 표시
    return f"Success: [{version}] {name} ({member_count} members)"

def process_version(version, db, parser):
    print(f"\n>>> [Version: {version}] Starting Full Discovery <<<")
    # 1단계: TOC 전체 뼈대 구축 (필터링 없이 모든 페이지 탐색)
    try:
        toc_data = fetch_toc(version)
        discover_nodes(toc_data, db, version)
    except Exception as e:
        print(f"[{version}] TOC Fatal Error: {e}")
        return

    # 2단계: 상세 수집 (is_scraped=0인 모든 Symbol들)
    db.cursor.execute("""
        SELECT id, name, url FROM symbols 
        WHERE version=? AND url IS NOT NULL AND is_scraped=0
    """, (version,))
    pending = db.cursor.fetchall()
    total = len(pending)
    
    if total == 0:
        print(f"[{version}] All items are fully scraped.")
        return

    print(f"[{version}] Step 2: Parallel Deep Scraping for {total} items...")
    processed = 0
    with ThreadPoolExecutor(max_workers=MAX_THREADS) as executor:
        futures = {executor.submit(scrape_worker, item, db, parser, version): item for item in pending}
        for future in as_completed(futures):
            processed += 1
            try:
                result = future.result()
                if processed % 10 == 0:
                    print(f"[{processed}/{total}] {result}")
            except Exception as e:
                print(f"[{processed}/{total}] ⚠️ {e}")

def main():
    db = DBManager("unity_api_nodes.db")
    parser = PageParser()
    for version in VERSIONS:
        process_version(version, db, parser)
    print("\n[COMPLETE] Global Unity API Database Building Finished!")
    db.close()

if __name__ == "__main__":
    main()
