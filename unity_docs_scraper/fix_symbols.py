"""
Symbol로 남은 노드 + 기존 Property만 있는 노드를 재수집.
- kind를 Class/Struct/Enum 등으로 업데이트
- Method/Property/Event 등 멤버를 올바르게 분류
- 429 에러 대응: 속도 조절 + 재시도
"""
import time
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed
from db_manager import DBManager
from page_parser import PageParser
import threading

MAX_THREADS = 8
REQUEST_DELAY = 0.2
RETRY_429_DELAY = 180  # 429 시 3분 대기
MAX_RETRIES = 3

session = requests.Session()

got_429 = threading.Event()

def rate_limited_fetch(parser, url, version):
    """429 발생 시 전체 3분 대기 후 재시도"""
    for attempt in range(MAX_RETRIES):
        # 다른 스레드에서 429 발생 시 같이 대기
        if got_429.is_set():
            got_429.wait()

        time.sleep(REQUEST_DELAY)
        soup = parser.fetch_soup(url, version=version, session=session)
        if soup is not None:
            return soup

        # None이면 429일 가능성 — 3분 대기
        if not got_429.is_set():
            got_429.set()
            print(f"\n[429] Rate limited. Waiting {RETRY_429_DELAY}s...", flush=True)
            time.sleep(RETRY_429_DELAY)
            got_429.clear()

    return None

def fix_worker(item, db, parser, version="6000.4"):
    sid, name, url = item

    if not url:
        return f"Skip (no url): {name}"

    try:
        soup = rate_limited_fetch(parser, url, version)
        if not soup:
            return f"Fetch Failed: {name}"

        details = parser.parse_class_page(soup, url)
        if not details:
            return f"Parse Failed: {name}"

        new_kind = details['category']
        new_summary = details['summary']

        # kind + summary 업데이트
        db.update_detail(sid, kind=new_kind, summary=new_summary)

        # 기존 자식 삭제 후 재삽입 (kind가 잘못된 기존 멤버 교체)
        with db.lock:
            db.cursor.execute("DELETE FROM symbols WHERE parent_id = ?", (sid,))
            db.conn.commit()

        member_count = 0
        for member in details['members']:
            m_signature = ""
            if member.get('link'):
                m_soup = rate_limited_fetch(parser, member['link'], version)
                if m_soup:
                    m_signature = parser.parse_signature(m_soup)

            db.insert_symbol({
                'parent_id': sid,
                'name': member['name'],
                'full_name': f"{name}.{member['name']}",
                'kind': member['member_type'],
                'summary': member['summary'],
                'signature': m_signature,
                'url': member.get('link'),
                'is_scraped': 1
            })
            member_count += 1

        # kind별 카운트
        kinds = {}
        for m in details['members']:
            k = m['member_type']
            kinds[k] = kinds.get(k, 0) + 1
        kind_str = ", ".join(f"{k}:{v}" for k, v in sorted(kinds.items()))

        return f"Fixed: {name} -> {new_kind} ({member_count} members: {kind_str})"

    except Exception as e:
        return f"Error: {name} - {e}"


def main():
    import sys
    from pathlib import Path

    mode = sys.argv[1] if len(sys.argv) > 1 else "symbols"

    db_path = Path(__file__).resolve().parent.parent / "unity_api_nodes.db"
    db = DBManager(str(db_path))
    parser = PageParser()

    if mode == "symbols":
        # Symbol kind인 것만 재수집
        with db.lock:
            db.cursor.execute("""
                SELECT id, name, url FROM symbols
                WHERE kind = 'Symbol' AND url IS NOT NULL
            """)
            pending = db.cursor.fetchall()
    elif mode == "all-classes":
        # Class/Struct/Enum 전부 재수집 (멤버 kind 수정용)
        with db.lock:
            db.cursor.execute("""
                SELECT id, name, url FROM symbols
                WHERE kind IN ('Class', 'Struct', 'Enum', 'Interface') AND url IS NOT NULL
            """)
            pending = db.cursor.fetchall()
    elif mode == "no-methods":
        # 자식이 전부 Property인 클래스 (Method가 없는 것)
        with db.lock:
            db.cursor.execute("""
                SELECT s.id, s.name, s.url FROM symbols s
                WHERE s.kind IN ('Class', 'Struct') AND s.url IS NOT NULL
                AND NOT EXISTS (
                    SELECT 1 FROM symbols c WHERE c.parent_id = s.id AND c.kind = 'Method'
                )
                AND EXISTS (
                    SELECT 1 FROM symbols c WHERE c.parent_id = s.id AND c.kind = 'Property'
                )
            """)
            pending = db.cursor.fetchall()
    else:
        print(f"Usage: python fix_symbols.py [symbols|all-classes|no-methods]")
        db.close()
        return

    total = len(pending)
    print(f"Mode: {mode} | Found {total} items to fix")

    if total == 0:
        print("Nothing to fix!")
        db.close()
        return

    fixed = 0
    failed = 0

    with ThreadPoolExecutor(max_workers=MAX_THREADS) as executor:
        futures = {executor.submit(fix_worker, item, db, parser): item for item in pending}
        for future in as_completed(futures):
            try:
                result = future.result()
                if 'Fixed' in result:
                    fixed += 1
                else:
                    failed += 1
                if (fixed + failed) % 10 == 0:
                    print(f"[{fixed + failed}/{total}] {result}", flush=True)
            except Exception as e:
                failed += 1
                print(f"Exception: {e}", flush=True)

    print(f"\nDone! Fixed: {fixed}, Failed: {failed}, Total: {total}")

    # 결과 확인
    with db.lock:
        db.cursor.execute("SELECT kind, COUNT(*) as c FROM symbols GROUP BY kind ORDER BY c DESC")
        for row in db.cursor.fetchall():
            print(f"  {row[0]}: {row[1]}")

    db.close()


if __name__ == "__main__":
    main()
