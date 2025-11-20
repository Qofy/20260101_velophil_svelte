import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { JSDOM } from 'jsdom';


describe('RxJS Integration for does not exists only', () => {
	let input: HTMLInputElement;
	let status: HTMLElement;
	let mockSpy: ReturnType<typeof vi.fn>;
	let fakeValidate: ReturnType<typeof vi.fn>;

	beforeEach(async () => {
		await initDom();
		input = document.getElementById('urlInput') as HTMLInputElement;
		status = document.getElementById('status')!;
		vi.useFakeTimers();
	});

	async function initDom() {
		const dom = new JSDOM(`
			<!DOCTYPE html>
			<html>
				<body>
					<input id="urlInput" />
					<div id="status"></div>
				</body>
			</html>
		`, {
			url: "http://localhost/",
		});

		global.window = dom.window as any;
		global.document = dom.window.document;
		global.HTMLElement = dom.window.HTMLElement;
		global.InputEvent = dom.window.InputEvent;

		// improve with spy fake
		fakeValidate = vi.fn(() => true); //  Always valid ValidationUrl

		// THEN import setupApp *after* mocking
		mockSpy = vi.fn(async (url: string) => ({
			exists: url.includes('does-not-exist') ? false : true,
			// exists: true,
			type: 'file',
		}));

		fakeValidate = vi.fn(() => true);

		const { setupApp } = await import('../src/main');

		// setupApp(mockSpy); // inject spy to RxJS version
		setupApp(mockSpy, fakeValidate);
	}
	function typeAndDispatch(url: string) {
		input.value = url;
		input.dispatchEvent(new Event('input', { bubbles: true }));
	}

	it('should show not found message for non-existent URL', async () => {
		typeAndDispatch('https://does-not-exist.com');
		await vi.runAllTimersAsync();
		expect(status.textContent).toBe('❌ https://does-not-exist.com does not exist.');
	});

	it('URL does not exist', async () => {
		typeAndDispatch('http://does-not-exist.com.com.mx/uiu/*(*');
		await vi.runAllTimersAsync();
		expect(status.textContent).toBe('❌ http://does-not-exist.com.com.mx/uiu/*(* does not exist.');
	});

	afterEach(() => {
		vi.useRealTimers();
	});
});
