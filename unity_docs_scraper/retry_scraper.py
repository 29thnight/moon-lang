import time
import sqlite3
from db_manager import DBManager
from page_parser import PageParser

def retry_failed_items():
    db = DBManager("unity_api_nodes.db")
    parser = PageParser()
    
    print("Checking for unscraped symbols in the database...")
    
    # 상세 페이지가 필요한데 수집이 안 된 항목들 조회
    db.cursor.execute("""
        SELECT id, name, full_name, kind, url 
        FROM symbols 
        WHERE url IS NOT NULL AND is_scraped = 0
    """)
    failed_items = db.cursor.fetchall()
    
    if not failed_items:
        print("No failed items found. Everything is up to date!")
        db.close()
        return

    print(f"Found {len(failed_items)} items to retry.")
    
    success_count = 0
    for sid, name, full_name, kind, url in failed_items:
        print(f"[*] Retrying: {full_name} ({url})")
        
        soup = parser.fetch_soup(url)
        if soup:
            details = parser.parse_class_page(soup, url)
            if details:
                # 상세 정보 업데이트 및 수집 완료 표시
                db.cursor.execute("""
                    UPDATE symbols 
                    SET kind=?, summary=?, is_scraped=1 
                    WHERE id=?
                """, (details['category'], details['summary'], sid))
                
                # 멤버 수집
                for member in details['members']:
                    db.insert_symbol({
                        'parent_id': sid,
                        'name': member['name'],
                        'full_name': f"{full_name}.{member['name']}",
                        'kind': member['member_type'],
                        'summary': member['summary'],
                        'url': member['link'],
                        'is_scraped': 1
                    })
                db.conn.commit()
                print(f"  └─ Success: Found {len(details['members'])} members")
                success_count += 1
            else:
                print(f"  └─ Failed: Could not parse details")
        else:
            print(f"  └─ Failed: Could not fetch page")
            
        time.sleep(0.1)

    print(f"\nRetry Complete! Successfully recovered {success_count} / {len(failed_items)} items.")
    db.close()

if __name__ == "__main__":
    retry_failed_items()
