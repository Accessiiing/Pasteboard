"""Image filter UI integration checks; uses isolated fixtures, never the real clipboard DB.
Run: python tests/image-filter.py
"""
from pathlib import Path
import json
from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'dist-out' / 'image-filter-review'
OUT.mkdir(parents=True, exist_ok=True)
BRIDGE = r'''
window.testEvents = {};
window.testCalls = [];
window.failPaste = false;
const now = Date.now();
const thumb = 'data:image/svg+xml;base64,' + btoa('<svg xmlns="http://www.w3.org/2000/svg" width="320" height="100"><rect width="320" height="100" fill="#ede8e2"/><circle cx="160" cy="50" r="30" fill="#cc785c"/></svg>');
window.testRecords = [
  { id: 1, kind: 'text', preview: '项目笔记：图片筛选验收', pinned: true },
  { id: 2, kind: 'image', preview: '风景截图', pinned: true, thumb },
  { id: 3, kind: 'link', preview: 'https://example.com', pinned: false },
  { id: 4, kind: 'image', preview: '界面截图', pinned: false, thumb },
  { id: 5, kind: 'text', preview: '待办：完成今天的项目检查', pinned: false },
].map(r => ({ created_at: now, last_used_at: now, ...r }));
window.emitTest = (name, payload) => (testEvents[name] || []).forEach(cb => cb({payload}));
window.__TAURI__ = {
  core: { invoke: async (cmd, args) => {
    testCalls.push({cmd, args});
    if (cmd === 'list_records') return structuredClone(testRecords);
    if (cmd === 'paste_record') {
      if (failPaste) throw '测试：图片文件不可读';
      if (!window.skipHiddenEvent) emitTest('panel-hidden');
    }
    if (cmd === 'hide_window') emitTest('panel-hidden');
    if (cmd === 'delete_record') testRecords = testRecords.filter(r => r.id !== args.id);
    if (cmd === 'pin_record') testRecords.find(r => r.id === args.id).pinned = args.pinned;
    if (cmd === 'save_record') return '测试导出目录';
    return null;
  }},
  event: { listen: async (name, cb) => {
    (testEvents[name] ||= []).push(cb);
    return () => {};
  }},
};
'''
checks = []
def passed(name):
    checks.append(name)
    print('PASS:', name)

with sync_playwright() as p:
    browser = p.chromium.launch(channel='msedge', headless=True)
    page = browser.new_page(viewport={'width': 350, 'height': 464}, device_scale_factor=2)
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.add_init_script(BRIDGE)
    page.goto((ROOT / 'ui' / 'index.html').as_uri())
    page.wait_for_load_state('networkidle')
    btn = page.locator('#btn-images')
    cards = page.locator('.card')
    def ids():
        return cards.evaluate_all('(cards) => cards.map(c => Number(c.dataset.id))')
    def state(value):
        expect(btn).to_have_attribute('data-state', value)
    def shot(name):
        page.mouse.move(5, 450)
        page.wait_for_timeout(180)
        page.screenshot(path=str(OUT / (name + '.png')))
    expect(cards).to_have_count(5)
    state('all')
    expect(btn).to_have_attribute('aria-pressed', 'false')
    shot('01-all')
    passed('初始显示文本、链接、图片，按钮未选中')

    btn.click()
    state('once')
    assert ids() == [2, 4]
    expect(page.locator('#footer')).to_contain_text('2 条图片')
    shot('02-once')
    passed('单次仅显示图片，保留固定与原始排序，计数正确')

    page.locator('#btn-search').click()
    page.locator('#search-input').fill('项目笔记')
    expect(cards).to_have_count(0)
    expect(page.locator('.empty')).to_have_text('没有匹配的记录')
    page.locator('#search-input').fill('界面')
    assert ids() == [4]
    page.evaluate("emitTest('tauri://blur')")
    state('once')
    assert ids() == [2, 4]
    passed('搜索与图片筛选取交集，普通 blur 不消耗单次状态')
    page.evaluate("emitTest('panel-hidden')")
    state('all')
    expect(cards).to_have_count(5)
    expect(page.locator('#search-bar')).not_to_have_class('search-bar show')
    passed('统一原生隐藏事件复位单次并清空搜索')

    btn.click()
    page.locator('.card[data-id="4"] .card-image').click()
    state('all')
    assert page.evaluate("testCalls.filter(c => c.cmd === 'paste_record').at(-1).args.id") == 4
    passed('筛选后粘贴正确图片 ID，原生隐藏先于命令返回仍能复位')

    page.evaluate('window.skipHiddenEvent = true')
    btn.click()
    page.locator('.card[data-id="2"] .card-image').click()
    state('all')
    passed('成功粘贴的前端兜底复位有效')
    page.evaluate('window.skipHiddenEvent = false')

    btn.dblclick()
    state('locked')
    shot('03-locked')
    colors = btn.evaluate('(el) => ({bg: getComputedStyle(el).backgroundColor, fg: getComputedStyle(el).color})')
    assert colors == {'bg': 'rgb(204, 120, 92)', 'fg': 'rgb(255, 255, 255)'}, colors
    passed('快速双击进入锁定，精确铜橙填充、白字')
    page.evaluate("emitTest('panel-hidden')")
    state('locked')
    page.locator('.card[data-id="4"] .card-image').click()
    state('locked')
    assert ids() == [2, 4]
    page.evaluate("testRecords.unshift({id: 6, kind:'text', preview:'新复制文本', created_at:Date.now()}); emitTest('records-updated')")
    expect(cards).to_have_count(2)
    state('locked')
    passed('锁定跨隐藏、粘贴、记录更新保留，新增文本被排除')
    btn.click()
    state('all')
    expect(cards).to_have_count(6)
    passed('锁定后再点恢复全部')

    btn.click()
    page.locator('.card[data-id="2"]').click(button='right')
    page.locator('.card[data-id="2"] [data-action="pin"]').click()
    state('once')
    expect(cards).to_have_count(2)
    page.locator('.card[data-id="4"]').click(button='right')
    page.locator('.card[data-id="4"] [data-action="save"]').click()
    state('once')
    passed('右键固定和保存不消耗单次状态')

    page.evaluate('window.failPaste = true')
    page.locator('.card[data-id="4"] .card-image').click()
    state('once')
    expect(page.locator('#toast')).to_contain_text('粘贴失败')
    expect(page.locator('.card.copied')).to_have_count(0)
    passed('粘贴失败保留筛选、清除点击态并提示错误')
    page.evaluate('window.failPaste = false')
    page.keyboard.press('Escape')
    state('all')
    passed('Escape 隐藏命令触发单次复位')

    page.evaluate("testRecords = testRecords.filter(r => r.kind !== 'image'); emitTest('records-updated')")
    expect(cards).to_have_count(4)
    btn.click()
    expect(page.locator('.empty')).to_have_text('暂无图片记录')
    expect(page.locator('#footer')).to_contain_text('0 条图片')
    btn.click()
    state('locked')
    page.evaluate("testRecords.push({id:7, kind:'image', preview:'新增截图', thumb:null, created_at:Date.now()}); emitTest('records-updated')")
    expect(cards).to_have_count(1)
    assert ids() == [7]
    passed('无图片空态正确，锁定时新图片立即出现，无缩略图仍可显示')
    page.locator('.card[data-id="7"]').click(button='right')
    page.locator('.card[data-id="7"] [data-action="delete"]').click()
    expect(page.locator('.empty')).to_have_text('暂无图片记录')
    state('locked')
    passed('删除最后一张图片后保持锁定并显示空态')

    btn.click()
    btn.focus()
    page.keyboard.press('Space')
    state('once')
    page.keyboard.press('Enter')
    state('locked')
    page.keyboard.press('Enter')
    state('all')
    passed('键盘 Space/Enter 可完成三态切换')
    assert not errors, errors
    assert page.locator('.header-actions').bounding_box()['x'] > page.locator('#view-clipboard .header-title').bounding_box()['x'] + page.locator('#view-clipboard .header-title').bounding_box()['width']
    assert page.evaluate('document.documentElement.scrollWidth') == 350
    passed('350px 布局无溢出、无脚本异常')
    browser.close()
(OUT / 'test-results.json').write_text(json.dumps({'passed': len(checks), 'checks': checks, 'errors': errors}, ensure_ascii=False, indent=2), encoding='utf-8')
print(f'{len(checks)} checks passed; artifacts: {OUT}')
