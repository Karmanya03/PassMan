// Background service worker for PassMann extension
chrome.runtime.onInstalled.addListener(() => {
  console.log('PassMann extension installed');
});

// Auto-lock vault when browser starts (clears any existing sessions)
chrome.runtime.onStartup.addListener(async () => {
  console.log('PassMann Background: Browser started - clearing session data');
  await chrome.storage.local.remove(['passmann_session_start']);
});

// Handle extension icon click
chrome.action.onClicked.addListener((tab) => {
  // Open popup (default behavior)
});

// Handle messages from content script or popup
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log('PassMann Background: Received message:', request.action, request);
  
  switch (request.action) {
    case 'autoFill':
      handleAutoFill(request.data, sender.tab);
      break;
    case 'getPageInfo':
      sendResponse({
        url: sender.tab.url,
        title: sender.tab.title
      });
      break;
    case 'savePassword':
      console.log('PassMann Background: Handling save password request');
      handleSavePassword(request.data, sendResponse);
      return true; // Keep message channel open for async response
    case 'checkCredentials':
      console.log('PassMann Background: Checking credentials for domain:', request.domain);
      handleCheckCredentials(request, sendResponse);
      return true; // Keep message channel open for async response
    case 'openPassMann':
      // Open extension popup
      console.log('PassMann Background: Opening popup');
      chrome.action.openPopup();
      break;
    case 'lockVault':
      // Immediately lock the vault
      console.log('PassMann Background: Locking vault');
      chrome.storage.local.remove(['passmann_session_start']);
      sendResponse({ success: true });
      break;
  }
});

async function handleAutoFill(data, tab) {
  try {
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: autoFillPage,
      args: [data.username, data.password]
    });
  } catch (error) {
    console.error('Auto-fill failed:', error);
  }
}

function autoFillPage(username, password) {
  // Find form fields
  const usernameSelectors = [
    'input[type="email"]',
    'input[type="text"][name*="user"]',
    'input[type="text"][name*="email"]',
    'input[type="text"][id*="user"]',
    'input[type="text"][id*="email"]',
    'input[name="username"]',
    'input[name="email"]',
    'input[id="username"]',
    'input[id="email"]'
  ];
  
  const passwordSelectors = [
    'input[type="password"]',
    'input[name="password"]',
    'input[id="password"]'
  ];
  
  // Fill username
  for (const selector of usernameSelectors) {
    const field = document.querySelector(selector);
    if (field && field.offsetParent !== null) { // Check if visible
      field.value = username;
      field.dispatchEvent(new Event('input', { bubbles: true }));
      field.dispatchEvent(new Event('change', { bubbles: true }));
      break;
    }
  }
  
  // Fill password
  for (const selector of passwordSelectors) {
    const field = document.querySelector(selector);
    if (field && field.offsetParent !== null) { // Check if visible
      field.value = password;
      field.dispatchEvent(new Event('input', { bubbles: true }));
      field.dispatchEvent(new Event('change', { bubbles: true }));
      break;
    }
  }
}

// Handle password save requests from content script
async function handleSavePassword(formData, sendResponse) {
  try {
    // Store the password data temporarily in session storage
    // The popup will handle the actual encryption and storage
    await chrome.storage.session.set({
      pendingSave: {
        site: formData.site,
        username: formData.username,
        password: formData.password,
        url: formData.url,
        timestamp: Date.now()
      }
    });
    
    console.log('Password save request received for:', formData.site);
    sendResponse({ success: true, message: 'Password queued for saving' });
  } catch (error) {
    console.error('Error handling save password request:', error);
    sendResponse({ success: false, message: 'Failed to process save request' });
  }
}

// Handle credential checking requests from content script
async function handleCheckCredentials(request, sendResponse) {
  try {
    // Get all saved entries from storage
    const result = await chrome.storage.local.get(['entries']);
    const entries = result.entries || [];
    const domain = request.domain;
    
    console.log('PassMann Background: Checking credentials for domain:', domain);
    console.log('PassMann Background: Total entries:', entries.length);
    
    // Find credentials that match the current domain
    const matchingCredentials = entries.filter(entry => {
      if (!entry) return false;
      
      const entryDomain = entry.service || entry.site || '';
      const entryUrl = entry.url || '';
      
      // Check for domain match (both ways to handle subdomains)
      const domainMatch = entryDomain.toLowerCase().includes(domain.toLowerCase()) ||
                         domain.toLowerCase().includes(entryDomain.toLowerCase());
      
      // Check for URL match
      const urlMatch = entryUrl && entryUrl.includes(domain);
      
      return domainMatch || urlMatch;
    });
    
    console.log('PassMann Background: Found matching credentials:', matchingCredentials.length);
    
    sendResponse({
      success: true,
      credentials: matchingCredentials.sort((a, b) => {
        // Sort by creation date, newest first
        return new Date(b.created || 0) - new Date(a.created || 0);
      })
    });
  } catch (error) {
    console.error('PassMann Background: Error checking credentials:', error);
    sendResponse({ success: false, error: error.message });
  }
}

// Set up periodic session cleanup
chrome.alarms.create('sessionCleanup', { periodInMinutes: 5 });
chrome.alarms.onAlarm.addListener(async (alarm) => {
  if (alarm.name === 'sessionCleanup') {
    try {
      const sessionData = await chrome.storage.local.get(['passmann_session_start']);
      if (sessionData.passmann_session_start) {
        const sessionAge = Date.now() - sessionData.passmann_session_start;
        const maxSessionTime = 15 * 60 * 1000; // 15 minutes
        
        if (sessionAge > maxSessionTime) {
          console.log('PassMann Background: Session expired, clearing data');
          await chrome.storage.local.remove(['passmann_session_start']);
        }
      }
    } catch (error) {
      console.error('PassMann Background: Session cleanup error:', error);
    }
  }
});
