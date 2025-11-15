// features/support/setup.ts
import { BeforeAll } from '@cucumber/cucumber';
import { JSDOM } from 'jsdom';

BeforeAll(async () => {
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

  // Import *after* DOM is available
	const { setupApp } = await import('../../src/main.ts');
  setupApp(); // attach RxJS to the DOM
});
