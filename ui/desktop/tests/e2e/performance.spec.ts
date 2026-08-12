import { test, expect } from './fixtures';

test.describe('Performance Tests', () => {
  test('measure end-to-end performance for prompt submission', async ({ goosePage }) => {
    // Start Playwright tracing to capture all performance data
    await goosePage.context().tracing.start({
      screenshots: true,
      snapshots: true,
      sources: true
    });

    console.log('\n=== Performance Test Started ===\n');

    // Mark: App ready
    await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 });
    await goosePage.evaluate(() => performance.mark('app-ready'));
    console.log('✓ App ready');

    // Prepare prompt
    const chatInput = await goosePage.waitForSelector('[data-testid="chat-input"]');
    const testPrompt = 'Write a haiku about testing software';
    await chatInput.fill(testPrompt);

    // Mark: Prompt submit
    await goosePage.evaluate(() => performance.mark('prompt-submit-start'));
    await chatInput.press('Enter');
    await goosePage.evaluate(() => performance.mark('prompt-submitted'));

    // Wait for loading indicator to appear and check if it's "loading conversation..."
    await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
      state: 'visible',
      timeout: 5000
    });

    const loadingText = await goosePage.locator('[data-testid="loading-indicator"]').textContent();
    if (loadingText?.includes('loading conversation')) {
      await goosePage.evaluate(() => performance.mark('loading-conversation-start'));
      console.log('✓ Loading conversation detected');

      // Wait for it to change or disappear
      await goosePage.waitForFunction(() => {
        const indicator = document.querySelector('[data-testid="loading-indicator"]');
        if (!indicator) return true; // Disappeared
        const text = indicator.textContent || '';
        return !text.includes('loading conversation'); // Changed to different state
      }, { timeout: 30000 });

      await goosePage.evaluate(() => performance.mark('loading-conversation-end'));
      console.log('✓ Loading conversation complete');
    }

    await goosePage.evaluate(() => performance.mark('loading-started'));

    // Monitor for first token (first visible response content)
    let firstTokenDetected = false;
    const checkForFirstToken = async () => {
      while (!firstTokenDetected) {
        try {
          const messageContainers = await goosePage.locator('[data-testid="message-container"]').all();
          if (messageContainers.length > 0) {
            const lastMessage = messageContainers[messageContainers.length - 1];
            const content = await lastMessage.textContent();
            if (content && content.trim().length > 0) {
              await goosePage.evaluate(() => performance.mark('first-token-received'));
              firstTokenDetected = true;
              console.log('✓ First token detected');
              break;
            }
          }
        } catch (e) {
          // Continue checking
        }
        await goosePage.waitForTimeout(50);
      }
    };

    // Start checking for first token
    const firstTokenPromise = checkForFirstToken();
    await firstTokenPromise;

    // Wait for response to complete
    await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
      state: 'hidden',
      timeout: 60000
    });
    await goosePage.evaluate(() => performance.mark('response-complete'));
    console.log('✓ Response complete');

    // Create performance measures
    await goosePage.evaluate(() => {
      // Measure loading conversation if it was detected
      const marks = performance.getEntriesByType('mark').map(m => m.name);
      if (marks.includes('loading-conversation-start') && marks.includes('loading-conversation-end')) {
        performance.measure('loading-conversation-duration', 'loading-conversation-start', 'loading-conversation-end');
      }

      performance.measure('time-to-prompt-submit', 'prompt-submit-start', 'prompt-submitted');
      performance.measure('time-to-first-token', 'prompt-submitted', 'first-token-received');
      performance.measure('time-to-complete-response', 'prompt-submitted', 'response-complete');
      performance.measure('streaming-duration', 'first-token-received', 'response-complete');
      performance.measure('total-interaction', 'prompt-submit-start', 'response-complete');
    });

    // Extract and display performance metrics
    const metrics = await goosePage.evaluate(() => {
      const measures = performance.getEntriesByType('measure');
      const result: Record<string, number> = {};
      measures.forEach(measure => {
        result[measure.name] = Math.round(measure.duration);
      });
      return result;
    });

    console.log('\n=== Performance Metrics ===');
    if (metrics['loading-conversation-duration']) {
      console.log(`Loading Conversation:       ${metrics['loading-conversation-duration']}ms`);
    }
    console.log(`Time to Submit Prompt:      ${metrics['time-to-prompt-submit']}ms`);
    console.log(`Time to First Token (TTFT): ${metrics['time-to-first-token']}ms`);
    console.log(`Time to Complete Response:  ${metrics['time-to-complete-response']}ms`);
    console.log(`Streaming Duration:         ${metrics['streaming-duration']}ms`);
    console.log(`Total Interaction Time:     ${metrics['total-interaction']}ms`);
    console.log('===========================\n');

    // Verify we got a response
    const response = await goosePage.locator('[data-testid="message-container"]').last();
    const responseText = await response.textContent();
    expect(responseText).toBeTruthy();
    expect(responseText!.length).toBeGreaterThan(0);

    // Assert performance thresholds
    expect(metrics['time-to-first-token']).toBeLessThan(10000); // First token in < 10s
    expect(metrics['time-to-complete-response']).toBeLessThan(60000); // Complete in < 60s

    // Stop tracing and save
    const tracePath = test.info().outputPath('trace.zip');
    await goosePage.context().tracing.stop({ path: tracePath });
    console.log(`✓ Performance trace saved to: ${tracePath}`);
    console.log(`  View with: npx playwright show-trace ${tracePath}\n`);

    // Attach metrics as JSON
    await test.info().attach('performance-metrics.json', {
      body: JSON.stringify(metrics, null, 2),
      contentType: 'application/json',
    });
  });

  test('measure cold start vs warm cache performance', async ({ goosePage }) => {
    await goosePage.context().tracing.start({ screenshots: true, snapshots: true });

    console.log('\n=== Cold vs Warm Performance ===\n');

    // Cold start measurement
    await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 });
    await goosePage.evaluate(() => performance.mark('app-ready'));

    // First prompt (cold)
    const chatInput = await goosePage.waitForSelector('[data-testid="chat-input"]');
    await chatInput.fill('Say hello');

    await goosePage.evaluate(() => performance.mark('cold-prompt-start'));
    await chatInput.press('Enter');

    await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
      state: 'visible',
      timeout: 5000
    });

    await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
      state: 'hidden',
      timeout: 60000
    });

    await goosePage.evaluate(() => {
      performance.mark('cold-prompt-complete');
      performance.measure('cold-prompt-duration', 'cold-prompt-start', 'cold-prompt-complete');
    });

    console.log('✓ Cold prompt complete');

    // Second prompt (warm)
    const chatInput2 = await goosePage.waitForSelector('[data-testid="chat-input"]');
    await chatInput2.fill('Say goodbye');

    await goosePage.evaluate(() => performance.mark('warm-prompt-start'));
    await chatInput2.press('Enter');

    await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
      state: 'visible',
      timeout: 5000
    });

    await goosePage.waitForSelector('[data-testid="loading-indicator"]', {
      state: 'hidden',
      timeout: 60000
    });

    await goosePage.evaluate(() => {
      performance.mark('warm-prompt-complete');
      performance.measure('warm-prompt-duration', 'warm-prompt-start', 'warm-prompt-complete');
    });

    console.log('✓ Warm prompt complete');

    // Extract metrics
    const metrics = await goosePage.evaluate(() => {
      const measures = performance.getEntriesByType('measure');
      const result: Record<string, number> = {};
      measures.forEach(measure => {
        result[measure.name] = Math.round(measure.duration);
      });
      return result;
    });

    const coldDuration = metrics['cold-prompt-duration'];
    const warmDuration = metrics['warm-prompt-duration'];
    const improvement = ((coldDuration - warmDuration) / coldDuration * 100).toFixed(1);

    console.log('\n=== Results ===');
    console.log(`Cold Prompt Duration: ${coldDuration}ms`);
    console.log(`Warm Prompt Duration: ${warmDuration}ms`);
    console.log(`Improvement:          ${improvement}%`);
    console.log('================\n');

    // Save trace
    const tracePath = test.info().outputPath('cold-vs-warm-trace.zip');
    await goosePage.context().tracing.stop({ path: tracePath });
    console.log(`✓ Trace saved to: ${tracePath}\n`);

    // Attach results
    await test.info().attach('cold-vs-warm.json', {
      body: JSON.stringify({
        coldDuration,
        warmDuration,
        improvement: `${improvement}%`
      }, null, 2),
      contentType: 'application/json',
    });
  });

  test('capture full performance profile with navigation timing', async ({ goosePage }) => {
    await goosePage.context().tracing.start({ screenshots: true, snapshots: true });

    console.log('\n=== Full Performance Profile ===\n');

    // Get navigation timing
    const navigationTiming = await goosePage.evaluate(() => {
      const perf = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
      return {
        domContentLoaded: Math.round(perf.domContentLoadedEventEnd - perf.fetchStart),
        loadComplete: Math.round(perf.loadEventEnd - perf.fetchStart),
        domInteractive: Math.round(perf.domInteractive - perf.fetchStart),
      };
    });

    console.log('Navigation Timing:');
    console.log(`  DOM Content Loaded: ${navigationTiming.domContentLoaded}ms`);
    console.log(`  DOM Interactive:    ${navigationTiming.domInteractive}ms`);
    console.log(`  Load Complete:      ${navigationTiming.loadComplete}ms`);

    // Wait for app ready
    await goosePage.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 });
    await goosePage.evaluate(() => performance.mark('app-interactive'));

    // Measure time from navigation to interactive
    const appReadyTime = await goosePage.evaluate(() => {
      const appInteractive = performance.getEntriesByName('app-interactive')[0];
      return Math.round(appInteractive.startTime);
    });

    console.log(`  App Interactive:    ${appReadyTime}ms\n`);

    // Submit a prompt and measure
    const chatInput = await goosePage.waitForSelector('[data-testid="chat-input"]');
    await chatInput.fill('Hello');

    await goosePage.evaluate(() => performance.mark('user-interaction-start'));
    await chatInput.press('Enter');

    await goosePage.waitForSelector('[data-testid="loading-indicator"]', { state: 'visible', timeout: 5000 });
    await goosePage.waitForSelector('[data-testid="loading-indicator"]', { state: 'hidden', timeout: 60000 });

    await goosePage.evaluate(() => {
      performance.mark('user-interaction-complete');
      performance.measure('user-interaction-duration', 'user-interaction-start', 'user-interaction-complete');
    });

    const interactionTime = await goosePage.evaluate(() => {
      const measure = performance.getEntriesByName('user-interaction-duration')[0];
      return Math.round(measure.duration);
    });

    console.log(`User Interaction Duration: ${interactionTime}ms\n`);

    // Get resource timing summary
    const resourceStats = await goosePage.evaluate(() => {
      const resources = performance.getEntriesByType('resource');
      const types: Record<string, number> = {};
      resources.forEach(resource => {
        const type = (resource as PerformanceResourceTiming).initiatorType;
        types[type] = (types[type] || 0) + 1;
      });
      return {
        total: resources.length,
        byType: types
      };
    });

    console.log('Resource Loading:');
    console.log(`  Total Resources: ${resourceStats.total}`);
    console.log(`  By Type:`, resourceStats.byType);
    console.log('\n==============================\n');

    // Save trace
    const tracePath = test.info().outputPath('full-profile-trace.zip');
    await goosePage.context().tracing.stop({ path: tracePath });
    console.log(`✓ Full trace saved to: ${tracePath}\n`);

    // Attach all metrics
    await test.info().attach('full-performance-profile.json', {
      body: JSON.stringify({
        navigationTiming,
        appReadyTime,
        interactionTime,
        resourceStats
      }, null, 2),
      contentType: 'application/json',
    });
  });
});
