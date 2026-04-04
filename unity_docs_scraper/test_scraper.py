import time
from toc_parser import fetch_toc, extract_items
from db_manager import DBManager
from page_parser import PageParser

def test_run():
    DB_NAME = "unity_api_test.db"
    db = DBManager(DB_NAME)
    parser = PageParser()
    
    # Manually defined classes for testing
    test_urls = [
        "GameObject",
        "Transform",
        "EditorWindow",
        "Vector3",
        "SceneManagement.SceneManager"
    ]
    
    print(f"Starting test scrape of {len(test_urls)} items...")
    
    for url_path in test_urls:
        print(f"Scraping {url_path}...")
        
        soup = parser.fetch_soup(url_path)
        if not soup:
            continue
            
        class_data = parser.parse_class_page(soup, url_path)
        if not class_data:
            print(f"Warning: Could not parse {url_path}")
            continue
            
        class_id = db.insert_class(class_data)
        if class_id:
            db.insert_members(class_id, class_data['members'])
            print(f"Stored {len(class_data['members'])} members for {url_path}.")
    
    print("\nTest Scrape Complete!")
    db.close()

if __name__ == "__main__":
    test_run()
