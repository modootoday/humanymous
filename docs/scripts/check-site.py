"""Read-only public-source and built-site checks; Python standard library only."""
import argparse
from html.parser import HTMLParser
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urljoin, urlsplit


ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = re.compile(r"(?:SoT-\d+|sot:root/|\.spec(?:\.local)?/|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|github_pat_[A-Za-z0-9_]+|ghp_[A-Za-z0-9]{30,})")


def pages():
    return sorted(p for p in ROOT.rglob('*.md') if p.name != 'AGENTS.md'
                  and not set(p.relative_to(ROOT).parts) & {'vendor', '_site', 'scripts', '.bundle'})


def frontmatter(path):
    text = path.read_text(encoding='utf-8')
    match = re.match(r'\A---\r?\n(.*?)\r?\n---\r?\n', text, re.S)
    if not match:
        raise ValueError(f'{path.relative_to(ROOT)}: missing explicit front matter')
    fields = dict(re.findall(r'^([a-z_]+):\s*(.*?)\s*$', match[1], re.M))
    return fields, text[match.end():]


def source():
    errors, urls = [], set()
    for path in pages():
        fields, body = frontmatter(path)
        rel = path.relative_to(ROOT)
        for name in ('layout', 'title', 'permalink', 'reviewed'):
            if not fields.get(name):
                errors.append(f'{rel}: missing {name}')
        if fields.get('layout') != 'doc':
            errors.append(f'{rel}: reader pages require doc layout')
        url = fields.get('permalink', '')
        if not url.startswith('/') or url in urls:
            errors.append(f'{rel}: invalid/duplicate permalink {url}')
        urls.add(url)
        if '**Diátaxis quadrant:**' not in body or '**Audience:**' not in body:
            errors.append(f'{rel}: missing purpose/audience')
        if FORBIDDEN.search(body):
            errors.append(f'{rel}: internal identifier or secret pattern')
        if 'standards' in rel.parts and fields.get('draft_status') != 'project-draft':
            errors.append(f'{rel}: missing project-draft status')
        if 'draft-v0' in rel.parts and fields.get('draft_version') != 'v0':
            errors.append(f'{rel}: missing v0 draft version')
        for target in re.findall(r'(?<!!)\[[^\]]+\]\(([^\s)]+)\)', body):
            parsed = urlsplit(target)
            if parsed.scheme or parsed.netloc or not parsed.path:
                continue
            local = (ROOT / parsed.path.lstrip('/') if parsed.path.startswith('/')
                     else path.parent / unquote(parsed.path)).resolve()
            if not local.is_relative_to(ROOT) or not local.exists():
                errors.append(f'{rel}: missing/escaping local source link {target}')
    return errors


class Document(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.ids, self.links, self.canonicals = set(), [], []
        self.main = self.h1 = self.lang = 0
        self.errors = []
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if 'id' in a:
            if a['id'] in self.ids:
                self.errors.append(f'duplicate id {a["id"]}')
            self.ids.add(a['id'])
        self.main += tag == 'main'
        self.h1 += tag == 'h1'
        self.lang += tag == 'html' and a.get('lang') == 'en'
        if tag == 'img' and 'alt' not in a:
            self.errors.append('image without alt')
        if tag == 'link' and a.get('rel') == 'canonical':
            self.canonicals.append(a.get('href', ''))
        for attr in ('href', 'src'):
            if a.get(attr):
                self.links.append(a[attr])


def built(site, baseurl):
    site = site.resolve()
    errors, docs = [], {}
    for path in site.rglob('*.html'):
        rel = path.relative_to(site).as_posix()
        url = '/' + rel.removesuffix('index.html') if path.name == 'index.html' else '/' + rel
        docs[url] = Document(path.read_text(encoding='utf-8'))
    for path in pages():
        fields, _ = frontmatter(path)
        url = fields['permalink']
        d = docs.get(url)
        if d is None:
            errors.append(f'missing rendered page {url}')
            continue
        if d.main != 1 or d.h1 != 1 or d.lang != 1:
            errors.append(f'{url}: expected one main/h1 and English language')
        expected = 'https://humanymous.net' + baseurl + url
        if d.canonicals != [expected]:
            errors.append(f'{url}: canonical {d.canonicals}, expected {expected}')
        errors.extend(f'{url}: {e}' for e in d.errors)
        text = (site / (url.lstrip('/') + 'index.html' if url.endswith('/') else url.lstrip('/'))).read_text(encoding='utf-8')
        if FORBIDDEN.search(text):
            errors.append(f'{url}: private identifier/secret leaked into HTML')
        for link in d.links:
            parsed = urlsplit(link)
            if parsed.scheme or parsed.netloc:
                continue
            full = urlsplit(urljoin('https://humanymous.net' + baseurl + url, link))
            target = unquote(full.path)
            if baseurl and not (target == baseurl or target.startswith(baseurl + '/')):
                errors.append(f'{url}: link escapes baseurl {link}')
                continue
            target = target[len(baseurl):] if baseurl else target
            local = site / target.lstrip('/')
            if target.endswith('/'):
                local /= 'index.html'
            if not local.is_file():
                errors.append(f'{url}: missing built target {link}')
            if full.fragment and target in docs and unquote(full.fragment) not in docs[target].ids:
                errors.append(f'{url}: missing anchor {link}')
    for forbidden in ('AGENTS.md', 'Gemfile', 'Gemfile.lock', 'reviews.json', 'scripts', 'assets/llms', 'assets/og', 'assets/screenshots', 'images'):
        if (site / forbidden).exists():
            errors.append(f'non-public/retired output exists: {forbidden}')
    return sorted(set(errors))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=['source', 'built'])
    parser.add_argument('--site', type=Path)
    parser.add_argument('--baseurl', default='')
    args = parser.parse_args()
    if args.mode == 'built' and args.site is None:
        parser.error('--site is required for built mode')
    try:
        errors = source() if args.mode == 'source' else built(args.site, args.baseurl)
    except (ValueError, OSError, KeyError) as e:
        errors = [str(e)]
    for error in errors:
        print(error, file=sys.stderr)
    print(f'Pages {args.mode}: {len(pages())} authored pages; {len(errors)} errors')
    return bool(errors)


if __name__ == '__main__':
    sys.exit(main())
