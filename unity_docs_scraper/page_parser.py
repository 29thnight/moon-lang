import requests
from bs4 import BeautifulSoup
import re
import time

class PageParser:
    BASE_URL_TEMPLATE = "https://docs.unity3d.com/{version}/Documentation/ScriptReference/"

    @staticmethod
    def fetch_soup(url_path, version="6000.4", session=None):
        base_url = PageParser.BASE_URL_TEMPLATE.format(version=version)
        url = base_url + url_path
        if not url.endswith(".html"):
            url += ".html"

        try:
            if session:
                response = session.get(url, timeout=15)
            else:
                response = requests.get(url, timeout=15)

            # 429 Too Many Requests — 대기 후 재시도
            if response.status_code == 429:
                retry_after = int(response.headers.get('Retry-After', 5))
                time.sleep(retry_after)
                return None

            if response.status_code != 200:
                return None
            return BeautifulSoup(response.text, 'html.parser')
        except Exception as e:
            return None

    @staticmethod
    def parse_class_page(soup, url_path):
        """클래스/구조체 메인 페이지에서 요약 및 멤버 목록 추출"""
        if not soup:
            return None

        data = {
            'name': '',
            'namespace': '',
            'category': 'Class',
            'summary': '',
            'url': url_path,
            'members': []
        }

        # 1. 클래스 이름 추출
        header = soup.find('h1', class_='heading')
        if not header:
            header = soup.find('h1')
        if header:
            data['name'] = header.get_text(strip=True)

        # 2. 종류(Class, Struct, Enum 등) 추출
        category_found = False

        # 패턴 A: <p class="cl ..."> 텍스트에서 추출
        for p in soup.find_all('p', class_='cl'):
            text = p.get_text(strip=True).lower()
            if 'enumeration' in text:
                data['category'] = 'Enum'; category_found = True
            elif 'struct' in text:
                data['category'] = 'Struct'; category_found = True
            elif 'interface' in text:
                data['category'] = 'Interface'; category_found = True
            elif 'class' in text:
                data['category'] = 'Class'; category_found = True
            if category_found:
                ns_match = re.search(r'in\s+([\w\.]+)', p.get_text(strip=True))
                if ns_match:
                    data['namespace'] = ns_match.group(1)
                break

        # 패턴 B: <div class="mb20"> → <div class="cl">
        if not category_found:
            namespace_div = soup.find('div', class_='mb20')
            if namespace_div:
                cl_div = namespace_div.find('div', class_='cl')
                if cl_div:
                    text = cl_div.get_text(strip=True)
                    match = re.match(r'(\w+)\s+in\s+([\w\.]+)', text)
                    if match:
                        data['category'] = match.group(1).capitalize()
                        data['namespace'] = match.group(2)
                        category_found = True

        # 패턴 C: 전체 텍스트에서 추론
        if not category_found:
            page_text = soup.get_text().lower()
            if 'enumeration' in page_text:
                data['category'] = 'Enum'
            elif 'struct in' in page_text:
                data['category'] = 'Struct'
            elif 'interface in' in page_text:
                data['category'] = 'Interface'

        # 3. 요약(Summary) 추출
        summary = ''
        skip_phrases = ['Thank you for helping', 'Suggest a change', 'Leave Feedback',
                        'Is something described', 'Success!', 'Submission failed']

        # 패턴 A: h3 "Description" 다음 p
        desc_h3 = None
        for h3 in soup.find_all('h3'):
            if h3.get_text(strip=True) == 'Description':
                desc_h3 = h3
                break
        if desc_h3:
            p = desc_h3.find_next('p')
            if p:
                text = p.get_text(strip=True)
                if not any(skip in text for skip in skip_phrases) and len(text) > 5:
                    summary = text

        # 패턴 B: div.description > p
        if not summary:
            desc_div = soup.find('div', class_='description')
            if desc_div:
                p = desc_div.find('p')
                if p:
                    summary = p.get_text(strip=True)

        # 패턴 C: 첫 번째 의미있는 p
        if not summary:
            for p in soup.find_all('p'):
                text = p.get_text(strip=True)
                if len(text) > 20 and not any(skip in text for skip in skip_phrases):
                    if 'in UnityEngine' not in text and 'in UnityEditor' not in text:
                        summary = text
                        break

        data['summary'] = summary[:500] if summary else ''

        # 4. 멤버 목록 수집 — **h2 AND h3** 섹션 아래 table.list
        #    Unity 6000.x는 h3를 사용, 이전 버전은 h2를 사용
        section_headers = soup.find_all(['h2', 'h3'])
        for heading in section_headers:
            section_title = heading.get_text(strip=True)

            # 섹션별 종류 판별
            section_lower = section_title.lower()
            kind = None
            if 'public method' in section_lower or section_lower == 'methods':
                kind = 'Method'
            elif 'static method' in section_lower:
                kind = 'Method'  # static은 별도 플래그로
            elif 'propert' in section_lower:
                kind = 'Property'
            elif 'event' in section_lower:
                kind = 'Event'
            elif 'message' in section_lower:
                kind = 'Message'
            elif 'field' in section_lower:
                kind = 'Field'
            elif 'enumerat' in section_lower or 'value' in section_lower:
                kind = 'EnumValue'
            elif 'constructor' in section_lower:
                kind = 'Constructor'
            elif 'operator' in section_lower:
                kind = 'Operator'

            if kind is None:
                continue  # Description, Inherited Members 등은 건너뜀

            is_static = 'static' in section_lower

            # 다음 table.list를 찾음
            table = heading.find_next('table')
            if not table:
                continue
            table_classes = table.get('class', [])
            if isinstance(table_classes, str):
                table_classes = table_classes.split()
            if 'list' not in table_classes:
                continue

            rows = table.find_all('tr')
            for row in rows:
                lbl = row.find('td', class_='lbl')
                desc = row.find('td', class_='desc')
                if lbl and desc:
                    link_tag = lbl.find('a')
                    if link_tag:
                        m_name = link_tag.get_text(strip=True)
                        m_link = link_tag.get('href')
                        m_summary = desc.get_text(strip=True)

                        data['members'].append({
                            'name': m_name,
                            'member_type': kind,
                            'is_static': is_static,
                            'summary': m_summary,
                            'link': m_link
                        })
        return data

    @staticmethod
    def parse_signature(soup):
        """멤버 상세 페이지에서 전체 선언문(Signature) 추출"""
        if not soup: return ""

        sig_div = soup.find('div', class_='signature')
        if not sig_div:
            sig_div = soup.find('div', class_='sig-kw')
        if not sig_div:
            pre = soup.find('pre', class_='codeExampleCS')
            if pre:
                return " ".join(pre.get_text().split())
        if not sig_div:
            code = soup.find('code')
            if code:
                text = code.get_text(strip=True)
                if 'public' in text or 'static' in text or '(' in text:
                    return " ".join(text.split())

        if sig_div:
            return " ".join(sig_div.get_text().split())
        return ""


if __name__ == "__main__":
    print("Testing parse on GameObject...")
    parser = PageParser()
    soup = parser.fetch_soup("GameObject", version="6000.4")
    if soup:
        result = parser.parse_class_page(soup, "GameObject")
        print(f"Name: {result['name']}")
        print(f"Category: {result['category']}")
        print(f"Summary: {result['summary'][:80]}")
        print(f"Members: {len(result['members'])}")
        kinds = {}
        for m in result['members']:
            k = m['member_type']
            kinds[k] = kinds.get(k, 0) + 1
        for k, v in sorted(kinds.items()):
            print(f"  {k}: {v}")
        print("Sample methods:")
        for m in result['members']:
            if m['member_type'] == 'Method':
                print(f"  Method: {m['name']}")
                if len([x for x in result['members'] if x['member_type']=='Method']) > 3:
                    break
