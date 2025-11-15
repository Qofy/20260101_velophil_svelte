import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { JSDOM } from 'jsdom';
import { setupApp } from '../src/main.ts';


describe('RxJS Integration', () => {
	let input: HTMLInputElement;
	let status: HTMLElement;
	let mockSpy: ReturnType<typeof vi.fn>;

	beforeEach(() => {

		initDom();
		input = document.getElementById('urlInput') as HTMLInputElement;
		status = document.getElementById('status')!;
		vi.useFakeTimers();
	});

	function initDom() {
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
		// setupApp(); // attach RxJS to the DOM, initialize RxJS listeners
		// improve with spy
		mockSpy = vi.fn((_url: string) => Promise.resolve({ exists: true, type: 'file' }));

		setupApp(mockSpy); // inject spy to RxJS version

	}
	function typeAndDispatch(url: string) {
		input.value = url;
		input.dispatchEvent(new Event('input', { bubbles: true }));
	}

	it('Valid URL points to a file', async () => {
		typeAndDispatch('https://example.com');
		await vi.runAllTimersAsync();
		expect(status.textContent).toContain('✅ https://example.com exists and points to a file.');
	});

	it('Typing invalid then valid URL', async () => {
		typeAndDispatch('not-a-url');
		expect(status.textContent).toBe('⚠️ Invalid URL format.');

		typeAndDispatch('https://example.com');
		await vi.runAllTimersAsync();
		expect(status.textContent).toContain('✅ https://example.com exists and points to a file.');
	});

	it('Typing a new URL clears the old result immediately', async () => {
		typeAndDispatch('https://first.com');
		vi.advanceTimersByTime(1000); // start debounce
		expect(status.textContent).toContain('⏳');

		typeAndDispatch('https://typing.com'); // immediately overrides
		expect(status.textContent).toBe('');

		await vi.runAllTimersAsync();
		expect(status.textContent).toContain('https://typing.com');
	});

	it('Old result is ignored if input changed', async () => {
		typeAndDispatch('https://first.com');
		vi.advanceTimersByTime(500); // half debounce
		typeAndDispatch('https://second.com');

		await vi.runAllTimersAsync();
		expect(status.textContent).toContain('https://second.com');
		expect(status.textContent).not.toContain('https://first.com');
	});

	it('Invalid URL should not trigger server check', async () => {
		typeAndDispatch('invalid-url');
		vi.advanceTimersByTime(2000); // nothing should change
		expect(status.textContent).toBe('⚠️ Invalid URL format.');
	});

	it('Spinner appears before result', async () => {
		typeAndDispatch('https://checking.com');
		vi.advanceTimersByTime(1000); // after debounce
		expect(status.textContent).toBe('⏳ Checking existence...');

		await vi.runAllTimersAsync();
		expect(status.textContent).toContain('✅ https://checking.com');
	});
	it('should handle typing invalid then valid URL', async () => {
		typeAndDispatch('invalid-url');
		expect(status.textContent).toBe('⚠️ Invalid URL format.');

		typeAndDispatch('https://example.com');
		await vi.runAllTimersAsync();
		expect(status.textContent).toContain('✅ https://example.com exists and points to a file.');
	});

	it('should cancel previous debounced calls on fast input changes', async () => {
		// const mockSpy = vi.fn((url: string) => Promise.resolve({ exists: true, type: 'file' }));
		// const mockSpy = vi.spyOn({ mockCheckUrlExists }, 'mockCheckUrlExists');

		// setupApp(mockSpy); // inject spy
		typeAndDispatch('https://one.com');
		vi.advanceTimersByTime(100);

		typeAndDispatch('https://two.com');
		vi.advanceTimersByTime(100);

		typeAndDispatch('https://three.com');
		await vi.runAllTimersAsync();

		expect(mockSpy).toHaveBeenCalledTimes(1);
		expect(mockSpy).toHaveBeenCalledWith('https://three.com');
	});

	it('should clear status when input is emptied', async () => {
		typeAndDispatch('https://example.com');
		vi.advanceTimersByTime(500); // mid-debounce

		typeAndDispatch(''); // clear input
		expect(status.textContent).toBe('');
	});
	it('should not re-fetch if the same URL is entered again', async () => {
		// const mockSpy = vi.spyOn({ mockCheckUrlExists }, 'mockCheckUrlExists');

		typeAndDispatch('https://repeat.com');
		await vi.runAllTimersAsync();

		expect(status.textContent).toContain('https://repeat.com');
		expect(mockSpy).toHaveBeenCalledTimes(1);

		// Re-enter the same URL
		typeAndDispatch('https://repeat.com');
		await vi.runAllTimersAsync();

		// No new call made
		expect(mockSpy).toHaveBeenCalledTimes(1);

		mockSpy.mockRestore();
	});


	afterEach(() => {
		vi.useRealTimers();
	});

});
