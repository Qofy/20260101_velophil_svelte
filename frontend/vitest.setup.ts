import { JSDOM } from 'jsdom';

const dom = new JSDOM(`
	<!DOCTYPE html>
	<html><body>
		<input id="urlInput" />
		<div id="status"></div>
	</body></html>
`);

global.window = dom.window as any;
global.document = dom.window.document;
global.HTMLElement = dom.window.HTMLElement;
global.InputEvent = dom.window.InputEvent;
