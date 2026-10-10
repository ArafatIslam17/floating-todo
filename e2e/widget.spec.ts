import { expect, test, type Page } from '@playwright/test';

// Wednesday 3 March 2027, 10:00:00 local time in the browser.
const START = new Date(2027, 2, 3, 10, 0, 0);

async function open(page: Page) {
  await page.clock.install({ time: START });
  await page.goto('/');
  await expect(page.getByLabel('New task title')).toBeVisible();
}

async function addScheduled(page: Page, title: string, time: string, lead?: string) {
  await page.getByLabel('New task title').fill(title);
  await page.getByRole('button', { name: 'Add date, time and notes' }).click();
  const details = page.locator('#quick-add-details');
  await details.getByLabel('Date').fill('2027-03-03');
  await details.getByLabel('Time').fill(time);
  if (lead) await details.getByLabel('Remind me').selectOption(lead);
  await page.getByLabel('New task title').press('Enter');
}

test('create, edit, complete, reopen, delete and undo', async ({ page }) => {
  await open(page);
  await expect(page.getByText('Nothing on your list.', { exact: false })).toBeVisible();

  await page.getByLabel('New task title').press('Enter');
  await expect(page.getByText('Enter a task title.')).toBeVisible();

  await addScheduled(page, 'Tutor Rafi', '10:25');
  const row = page.getByRole('listitem').filter({ hasText: 'Tutor Rafi' });
  await expect(row).toContainText('Today 10:25');
  await expect(row).toContainText('in 25 min');

  // Edit: rename and reschedule.
  await row.hover();
  await row.getByRole('button', { name: 'Edit Tutor Rafi' }).click();
  await page.getByLabel('Title', { exact: true }).fill('Tutor Rafi, chapter 5');
  await page.getByRole('form', { name: /Edit/ }).getByLabel('Time').fill('12:00');
  await page.getByRole('button', { name: 'Save' }).click();
  const edited = page.getByRole('listitem').filter({ hasText: 'chapter 5' });
  await expect(edited).toContainText('in 2 hr');

  // Complete and reopen.
  await edited.getByRole('checkbox').check();
  await expect(page.getByText('All done.', { exact: false })).toBeVisible();
  await page.getByRole('button', { name: 'Completed (1)' }).click();
  await page.getByRole('checkbox', { name: 'Reopen Tutor Rafi, chapter 5' }).uncheck();
  await expect(page.getByRole('list', { name: 'Open tasks' })).toContainText('chapter 5');

  // Delete and undo.
  const again = page.getByRole('list', { name: 'Open tasks' }).getByRole('listitem').first();
  await again.hover();
  await again.getByRole('button', { name: /^Delete/ }).click();
  await expect(page.getByText('Deleted “Tutor Rafi, chapter 5”')).toBeVisible();
  await page.getByRole('button', { name: 'Undo' }).click();
  await expect(page.getByRole('list', { name: 'Open tasks' })).toContainText('chapter 5');
});

test('due time: banner, snooze keeps the deadline, dismiss keeps the task', async ({ page }) => {
  await open(page);
  await addScheduled(page, 'Submit DSP report', '10:02');
  const row = page.getByRole('listitem').filter({ hasText: 'Submit DSP report' });
  await expect(row).toContainText('in 2 min');

  await page.clock.fastForward('02:01');
  const banner = page.getByRole('alert', { name: 'Reminder' });
  await expect(banner).toContainText('Submit DSP report');
  await expect(row).toHaveAttribute('data-state', 'now');

  await banner.getByRole('button', { name: 'Snooze 10 min' }).click();
  await expect(banner).toBeHidden();
  await expect(row).toContainText('Snoozed until 10:12');
  await expect(row).toContainText('Today 10:02'); // deadline unchanged

  await page.clock.fastForward('10:00');
  await expect(banner).toBeVisible();
  await expect(row).toHaveAttribute('data-state', 'overdue');
  await expect(row).toContainText('Overdue 10 min');
  await expect(page.getByText('1 overdue')).toBeVisible();

  await page.keyboard.press('Alt+d');
  await expect(banner).toBeHidden();
  await expect(row).toBeVisible();
});

test('completing from the alert banner clears it', async ({ page }) => {
  await open(page);
  await addScheduled(page, 'Call tuition guardian', '10:01');
  await page.clock.fastForward('01:01');
  const banner = page.getByRole('alert', { name: 'Reminder' });
  await banner.getByRole('button', { name: 'Complete' }).click();
  await expect(banner).toBeHidden();
  await expect(page.getByText('All done.', { exact: false })).toBeVisible();
});

test('lead reminder fires before the deadline', async ({ page }) => {
  await open(page);
  await addScheduled(page, 'Class rep meeting', '10:20', '15');
  await page.clock.fastForward('05:01');
  const banner = page.getByRole('alert', { name: 'Reminder' });
  await expect(banner).toContainText('Class rep meeting');
  await expect(banner).toContainText('Due in 15 min');
});

test('keyboard-only flow', async ({ page }) => {
  await open(page);
  await page.keyboard.press('Control+n');
  await page.keyboard.type('Read paper');
  await page.keyboard.press('Enter');
  await page.keyboard.type('Email lab');
  await page.keyboard.press('Enter');
  const list = page.getByRole('list', { name: 'Open tasks' });
  await list.getByRole('listitem').first().focus();
  await page.keyboard.press('ArrowDown');
  await expect(list.getByRole('listitem').nth(1)).toBeFocused();
  await page.keyboard.press('Space');
  await expect(list.getByRole('listitem')).toHaveCount(1);
  await list.getByRole('listitem').first().focus();
  await page.keyboard.press('Delete');
  await expect(page.getByText(/Deleted/)).toBeVisible();
  await page.keyboard.press('Control+z');
  await expect(list.getByRole('listitem')).toHaveCount(1);
});

test('narrow widget: no horizontal overflow with long titles', async ({ page }) => {
  await page.setViewportSize({ width: 300, height: 480 });
  await open(page);
  await page.getByLabel('New task title').fill(
    'Prepare the comparative evaluation tables for the bandwidth extension manuscript revision and send to co-authors',
  );
  await page.getByLabel('New task title').press('Enter');
  await addScheduled(page, 'Antidisestablishmentarianism_without_any_spaces_to_break_on_at_all', '11:45', '30');
  await page.clock.runFor(1000); // let entry animations finish (GSAP runs on the fake clock)
  const overflow = await page.locator('.panel').evaluate((el) => el.scrollWidth - el.clientWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.screenshot({ path: 'test-results/narrow-300px.png' });
});
