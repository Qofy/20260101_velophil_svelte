import { Given, When, Then } from '@cucumber/cucumber';
import assert from 'assert';

// **
// **
// **  Given I enter the URL ...
// **
// **

Given('I enter the URL {string}', async function (url: string) {
	const input = document.getElementById('urlInput') as HTMLInputElement;
	input.value = url;

	// use the JSDOM-scoped Event constructor
	const event = new (window as any).Event('input', { bubbles: true });
	input.dispatchEvent(event);
});

Given('I wait {float} second', async function (seconds: number) {
	await new Promise((resolve) => setTimeout(resolve, seconds * 1000));
});


// **
// **
// **  Wait helpers
// **
// **

When('I wait {float} seconds', async function (seconds: number) {
	await new Promise((resolve) => setTimeout(resolve, seconds * 1000));
});

When('I wait {int} milliseconds', async function (ms: number) {
	await new Promise((resolve) => setTimeout(resolve, ms));
});
When('I wait for the check to complete', async function () {
	await new Promise((resolve) => setTimeout(resolve, 1600)); // 1000ms debounce + 500ms mock delay + margin
});

// **
// **
// **  Status assertion
// **
// **

Then('I should see {string}', async function (expected: string) {
	const timeout = 1000;
	const start = Date.now();

	while (Date.now() - start < timeout) {
		const status = document.getElementById('status')!;
		const actual = status.textContent?.trim() || '';

		if (actual.includes(expected)) return;

		await new Promise((r) => setTimeout(r, 50));
	}

	const status = document.getElementById('status')!;
	const actual = status.textContent?.trim() || '';
	throw new Error(`Expected status to include "${expected}", but got: "${actual}"`);
});

Then('I should immediately see an empty status', function () {
	const status = document.getElementById('status')!;
	const actual = status.textContent?.trim() || '';
	assert.strictEqual(actual, '', `Expected status to be empty, but got: "${actual}"`);
});

Then('I should not see {string}', async function (expected: string) {
	const timeout = 1000;
	const start = Date.now();

	while (Date.now() - start < timeout) {
		const status = document.getElementById('status')!;
		const actual = status.textContent?.trim() || '';

		if (!actual.includes(expected)) return;

		await new Promise((r) => setTimeout(r, 50));
	}

	const status = document.getElementById('status')!;
	const actual = status.textContent?.trim() || '';
	throw new Error(`Expected status to include "${expected}", but got: "${actual}"`);
});
