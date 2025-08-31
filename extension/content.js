// Content script for PassMannn - detects login forms and enables auto-fill with save popup
(function() {
  'use strict';
  
  console.log('PassMannn: Content script loaded');
  
  let currentFormData = null;
  let savePopup = null;
  let autoFillPopup = null;
  
  // Auto-fill functions
  function checkForSavedCredentials() {
    console.log('PassMannn: Checking for saved credentials for:', window.location.hostname);
    
    // Send message to background script to check for saved credentials
    chrome.runtime.sendMessage({
      action: 'checkCredentials',
      domain: window.location.hostname,
      url: window.location.href
    }, (response) => {
      if (response && response.credentials && response.credentials.length > 0) {
        console.log('PassMann: Found saved credentials:', response.credentials.length);
        showAutoFillPopup(response.credentials);
      } else {
        console.log('PassMann: No saved credentials found for this site');
      }
    });
  }
  
  function showAutoFillPopup(credentials) {
    // Don't show auto-fill popup if save popup is already showing
    if (savePopup) return;
    
    // Remove existing auto-fill popup if any
    removeAutoFillPopup();
    
    const primaryCredential = credentials[0]; // Use the first/most recent credential
    
    console.log('PassMann: Showing auto-fill popup for:', primaryCredential.service || primaryCredential.site);
    
    // Create auto-fill popup
    const popup = document.createElement('div');
    popup.id = 'PassMann-autofill-popup';
    popup.style.cssText = `
      position: fixed !important;
      top: 20px !important;
      right: 20px !important;
      z-index: 2147483647 !important;
      background: #ffffff !important;
      border: 1px solid #dadce0 !important;
      border-radius: 8px !important;
      box-shadow: 0 4px 16px rgba(0,0,0,0.2) !important;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
      font-size: 14px !important;
      width: 300px !important;
      max-width: 300px !important;
      animation: PassMannSlideIn 0.3s ease-out !important;
      pointer-events: auto !important;
    `;
    
    const serviceName = primaryCredential.service || primaryCredential.site || window.location.hostname;
    const maskedPassword = '•'.repeat(Math.min(primaryCredential.password.length, 8));
    
    popup.innerHTML = `
      <div style="
        display: flex !important;
        align-items: center !important;
        padding: 16px 16px 12px 16px !important;
        border-bottom: 1px solid #f0f0f0 !important;
      ">
        <div style="
          width: 24px !important;
          height: 24px !important;
          background: linear-gradient(135deg, #34A853, #4285F4) !important;
          border-radius: 4px !important;
          display: flex !important;
          align-items: center !important;
          justify-content: center !important;
          margin-right: 12px !important;
          font-size: 12px !important;
        ">🔑</div>
        <div>
          <div style="font-weight: 500 !important; color: #202124 !important;">Use saved password?</div>
          <div style="font-size: 12px !important; color: #5f6368 !important; margin-top: 2px !important;">PassMann can fill your ${serviceName} password</div>
        </div>
      </div>
      <div style="padding: 12px 16px !important;">
        <div style="
          background: #f8f9fa !important;
          border: 1px solid #e8eaed !important;
          border-radius: 4px !important;
          padding: 8px 12px !important;
          margin-bottom: 12px !important;
          font-size: 13px !important;
        ">
          <div style="color: #5f6368 !important; font-size: 11px !important; margin-bottom: 4px !important;">USERNAME</div>
          <div style="color: #202124 !important; font-weight: 500 !important;">${primaryCredential.username}</div>
        </div>
        <div style="
          background: #f8f9fa !important;
          border: 1px solid #e8eaed !important;
          border-radius: 4px !important;
          padding: 8px 12px !important;
          margin-bottom: 16px !important;
          font-size: 13px !important;
        ">
          <div style="color: #5f6368 !important; font-size: 11px !important; margin-bottom: 4px !important;">PASSWORD</div>
          <div style="color: #202124 !important;">${maskedPassword}</div>
        </div>
        <div style="display: flex !important; gap: 8px !important;">
          <button id="PassMann-autofill-btn" style="
            flex: 1 !important;
            background: #1a73e8 !important;
            color: white !important;
            border: none !important;
            border-radius: 4px !important;
            padding: 8px 16px !important;
            font-size: 13px !important;
            font-weight: 500 !important;
            cursor: pointer !important;
            transition: background-color 0.2s !important;
          ">
            Auto-fill
          </button>
          <button id="PassMann-autofill-cancel-btn" style="
            flex: 1 !important;
            background: transparent !important;
            color: #1a73e8 !important;
            border: 1px solid #dadce0 !important;
            border-radius: 4px !important;
            padding: 8px 16px !important;
            font-size: 13px !important;
            font-weight: 500 !important;
            cursor: pointer !important;
            transition: background-color 0.2s !important;
          ">
            Not now
          </button>
        </div>
      </div>
    `;
    
    // Append to body
    document.body.appendChild(popup);
    autoFillPopup = popup;
    
    console.log('PassMann: Auto-fill popup created and added to DOM');
    
    // Add event listeners
    const autoFillBtn = document.getElementById('PassMann-autofill-btn');
    const cancelBtn = document.getElementById('PassMann-autofill-cancel-btn');
    
    if (autoFillBtn) {
      autoFillBtn.addEventListener('click', () => {
        console.log('PassMann: Auto-fill button clicked');
        fillCredentials(primaryCredential);
        removeAutoFillPopup();
      });
    }
    
    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => {
        console.log('PassMann: Auto-fill cancel button clicked');
        removeAutoFillPopup();
      });
    }
    
    // Auto-dismiss after 15 seconds
    setTimeout(() => {
      console.log('PassMann: Auto-dismissing auto-fill popup after 15 seconds');
      removeAutoFillPopup();
    }, 15000);
  }
  
  function removeAutoFillPopup() {
    if (autoFillPopup) {
      autoFillPopup.style.animation = 'PassMannSlideOut 0.3s ease-in forwards';
      setTimeout(() => {
        if (autoFillPopup && autoFillPopup.parentNode) {
          autoFillPopup.parentNode.removeChild(autoFillPopup);
        }
        autoFillPopup = null;
      }, 300);
    }
  }
  
  function fillCredentials(credential) {
    console.log('PassMann: Filling credentials for:', credential.username);
    
    // Find username field
    let usernameField = null;
    const usernameSelectors = [
      'input[type="email"]:not([disabled])',
      'input[name*="username"]:not([disabled])',
      'input[name*="email"]:not([disabled])',
      'input[name*="user"]:not([disabled])',
      'input[id*="username"]:not([disabled])',
      'input[id*="email"]:not([disabled])',
      'input[autocomplete="username"]:not([disabled])',
      'input[autocomplete="email"]:not([disabled])',
      'input[type="text"]:not([disabled])'
    ];
    
    for (let selector of usernameSelectors) {
      const field = document.querySelector(selector);
      if (field && isVisible(field)) {
        usernameField = field;
        break;
      }
    }
    
    // Find password field
    const passwordField = document.querySelector('input[type="password"]:not([disabled])');
    
    // Fill the fields
    if (usernameField) {
      usernameField.value = credential.username;
      usernameField.dispatchEvent(new Event('input', { bubbles: true }));
      usernameField.dispatchEvent(new Event('change', { bubbles: true }));
      console.log('PassMann: Username field filled');
    }
    
    if (passwordField) {
      passwordField.value = credential.password;
      passwordField.dispatchEvent(new Event('input', { bubbles: true }));
      passwordField.dispatchEvent(new Event('change', { bubbles: true }));
      console.log('PassMann: Password field filled');
    }
    
    if (usernameField || passwordField) {
      console.log('PassMann: Credentials auto-filled successfully');
    } else {
      console.log('PassMann: Could not find fields to fill');
    }
  }
  
  // Monitor forms for submission and button clicks
  function setupFormMonitoring() {
    console.log('PassMann: Setting up form monitoring');
    
    // Monitor all forms
    const forms = document.querySelectorAll('form');
    forms.forEach((form, index) => {
      console.log(`PassMann: Monitoring form ${index + 1}`);
      
      form.addEventListener('submit', function(event) {
        console.log('PassMann: Form submit detected');
        handleFormAction();
      }, true);
    });
    
    // Monitor all buttons that might be login/register buttons
    const buttons = document.querySelectorAll('button, input[type="submit"], input[type="button"], a[role="button"]');
    buttons.forEach(button => {
      const buttonText = (button.textContent || button.value || button.innerText || '').toLowerCase().trim();
      const buttonId = (button.id || '').toLowerCase();
      const buttonClass = (button.className || '').toLowerCase();
      
      // Enhanced button detection with more patterns
      if (buttonText.includes('sign') || buttonText.includes('log') || buttonText.includes('submit') || 
          buttonText.includes('enter') || buttonText.includes('register') || buttonText.includes('continue') ||
          buttonText.includes('next') || buttonText.includes('create') || buttonText.includes('join') ||
          buttonId.includes('login') || buttonId.includes('signin') || buttonId.includes('submit') ||
          buttonId.includes('register') || buttonId.includes('signup') ||
          buttonClass.includes('login') || buttonClass.includes('signin') || buttonClass.includes('submit') ||
          buttonClass.includes('register') || buttonClass.includes('signup')) {
        
        console.log('PassMann: Monitoring button:', buttonText || buttonId || button.className || 'unnamed button');
        
        button.addEventListener('click', function(event) {
          console.log('PassMann: Login/register button clicked:', buttonText || buttonId);
          
          // Small delay to allow form processing
          setTimeout(() => {
            handleFormAction();
          }, 300);
        }, true);
      }
    });
    
    // Monitor for dynamically added forms and buttons
    const observer = new MutationObserver(function(mutations) {
      mutations.forEach(function(mutation) {
        mutation.addedNodes.forEach(function(node) {
          if (node.nodeType === Node.ELEMENT_NODE) {
            // Check for new forms
            if (node.tagName === 'FORM') {
              console.log('PassMann: New form detected');
              setupFormMonitoring();
            }
            
            // Check for new forms in added content
            const newForms = node.querySelectorAll ? node.querySelectorAll('form') : [];
            if (newForms.length > 0) {
              console.log('PassMann: New forms detected in added content');
              setupFormMonitoring();
            }
          }
        });
      });
    });
    
    observer.observe(document.body, {
      childList: true,
      subtree: true
    });
  }
  
  // Handle form action (submission or button click)
  function handleFormAction() {
    console.log('PassMann: Handling form action - checking for password data');
    
    const formData = extractFormData();
    
    if (formData && formData.password) {
      console.log('PassMann: Valid password data found, creating save popup');
      createSavePopup(formData);
    } else {
      console.log('PassMann: No valid password data found');
    }
  }
  
  // Extract form data from login forms - simplified and reliable
  function extractFormData() {
    console.log('PassMann: Extracting form data...');
    
    // Find password field (must exist and have value)
    const passwordField = document.querySelector('input[type="password"]:not([disabled])');
    if (!passwordField || !passwordField.value.trim()) {
      console.log('PassMann: No password field with value found');
      return null;
    }
    
    console.log('PassMann: Password field found with value');
    
    // Find username field - try multiple approaches
    let usernameField = null;
    let usernameValue = '';
    
    // Try email field first
    usernameField = document.querySelector('input[type="email"]:not([disabled])');
    if (usernameField && usernameField.value.trim()) {
      usernameValue = usernameField.value.trim();
      console.log('PassMann: Found email field');
    } else {
      // Try text inputs with common names/ids/autocomplete
      const selectors = [
        'input[name*="username"]:not([disabled])',
        'input[name*="email"]:not([disabled])',
        'input[name*="user"]:not([disabled])',
        'input[name*="login"]:not([disabled])',
        'input[id*="username"]:not([disabled])',
        'input[id*="email"]:not([disabled])',
        'input[id*="user"]:not([disabled])',
        'input[id*="login"]:not([disabled])',
        'input[autocomplete="username"]:not([disabled])',
        'input[autocomplete="email"]:not([disabled])',
        'input[type="text"]:not([disabled])',
        'input[type="tel"]:not([disabled])'
      ];
      
      for (let selector of selectors) {
        const field = document.querySelector(selector);
        if (field && field.value.trim()) {
          usernameField = field;
          usernameValue = field.value.trim();
          console.log('PassMann: Found username field with selector:', selector);
          break;
        }
      }
      
      // If still no username, try to find any visible text input with a value
      if (!usernameValue) {
        const allTextInputs = document.querySelectorAll('input[type="text"]:not([disabled]), input:not([type]):not([disabled])');
        for (let input of allTextInputs) {
          if (input.value.trim() && input !== passwordField) {
            const isVisible = input.offsetParent !== null && 
                            getComputedStyle(input).visibility !== 'hidden' &&
                            getComputedStyle(input).display !== 'none';
            if (isVisible) {
              usernameField = input;
              usernameValue = input.value.trim();
              console.log('PassMann: Found fallback username field');
              break;
            }
          }
        }
      }
    }
    
    if (!usernameValue) {
      console.log('PassMann: No username field with value found');
      return null;
    }
    
    const formData = {
      site: window.location.hostname,
      username: usernameValue,
      password: passwordField.value,
      url: window.location.href
    };
    
    console.log('PassMann: Successfully extracted form data for:', formData.site, formData.username);
    return formData;
  }
  
  // Create Chrome-style save password popup
  function createSavePopup(formData) {
    console.log('PassMann: Creating save popup for:', formData.site);
    
    // Remove existing popup if any
    removeSavePopup();
    
    // Create popup container
    const popup = document.createElement('div');
    popup.id = 'PassMann-save-popup';
    popup.style.cssText = `
      position: fixed !important;
      top: 80px !important;
      right: 20px !important;
      z-index: 2147483647 !important;
      background: #ffffff !important;
      border: 1px solid #dadce0 !important;
      border-radius: 8px !important;
      box-shadow: 0 4px 16px rgba(0,0,0,0.2) !important;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif !important;
      font-size: 14px !important;
      width: 320px !important;
      max-width: 320px !important;
      animation: PassMannSlideIn 0.3s ease-out !important;
      pointer-events: auto !important;
    `;
    
    popup.innerHTML = `
      <div style="
        display: flex !important;
        align-items: center !important;
        padding: 16px 16px 12px 16px !important;
        border-bottom: 1px solid #f0f0f0 !important;
      ">
        <div style="
          width: 24px !important;
          height: 24px !important;
          background: linear-gradient(135deg, #3B82F6, #8B5CF6) !important;
          border-radius: 4px !important;
          display: flex !important;
          align-items: center !important;
          justify-content: center !important;
          margin-right: 12px !important;
          font-size: 12px !important;
        "><img src="${chrome.runtime.getURL('passman1-1.PNG')}" style="width: 16px; height: 16px;"></div>
        <div>
          <div style="font-weight: 500 !important; color: #202124 !important;">Save password?</div>
          <div style="font-size: 12px !important; color: #5f6368 !important; margin-top: 2px !important;">PassMann can save this password for ${formData.site}</div>
        </div>
      </div>
      <div style="padding: 12px 16px !important;">
        <div style="
          background: #f8f9fa !important;
          border: 1px solid #e8eaed !important;
          border-radius: 4px !important;
          padding: 8px 12px !important;
          margin-bottom: 12px !important;
          font-size: 13px !important;
        ">
          <div style="color: #5f6368 !important; font-size: 11px !important; margin-bottom: 4px !important;">USERNAME</div>
          <div style="color: #202124 !important; font-weight: 500 !important;">${formData.username}</div>
        </div>
        <div style="
          background: #f8f9fa !important;
          border: 1px solid #e8eaed !important;
          border-radius: 4px !important;
          padding: 8px 12px !important;
          margin-bottom: 16px !important;
          font-size: 13px !important;
        ">
          <div style="color: #5f6368 !important; font-size: 11px !important; margin-bottom: 4px !important;">PASSWORD</div>
          <div style="color: #202124 !important;">••••••••</div>
        </div>
        <div style="display: flex !important; gap: 8px !important;">
          <button id="PassMann-save-btn" style="
            flex: 1 !important;
            background: #1a73e8 !important;
            color: white !important;
            border: none !important;
            border-radius: 4px !important;
            padding: 8px 16px !important;
            font-size: 13px !important;
            font-weight: 500 !important;
            cursor: pointer !important;
            transition: background-color 0.2s !important;
          ">
            Save
          </button>
          <button id="PassMann-cancel-btn" style="
            flex: 1 !important;
            background: transparent !important;
            color: #1a73e8 !important;
            border: 1px solid #dadce0 !important;
            border-radius: 4px !important;
            padding: 8px 16px !important;
            font-size: 13px !important;
            font-weight: 500 !important;
            cursor: pointer !important;
            transition: background-color 0.2s !important;
          ">
            Cancel
          </button>
        </div>
      </div>
    `;
    
    // Add animation styles if not already added
    if (!document.getElementById('PassMann-styles')) {
      const styles = document.createElement('style');
      styles.id = 'PassMann-styles';
      styles.textContent = `
        @keyframes PassMannSlideIn {
          from {
            transform: translateX(100%) !important;
            opacity: 0 !important;
          }
          to {
            transform: translateX(0) !important;
            opacity: 1 !important;
          }
        }
        @keyframes PassMannSlideOut {
          from {
            transform: translateX(0) !important;
            opacity: 1 !important;
          }
          to {
            transform: translateX(100%) !important;
            opacity: 0 !important;
          }
        }
      `;
      document.head.appendChild(styles);
    }
    
    // Append to body
    document.body.appendChild(popup);
    savePopup = popup;
    
    console.log('PassMann: Popup created and added to DOM');
    
    // Add event listeners
    const saveBtn = document.getElementById('PassMann-save-btn');
    const cancelBtn = document.getElementById('PassMann-cancel-btn');
    
    if (saveBtn) {
      saveBtn.addEventListener('click', () => {
        console.log('PassMann: Save button clicked');
        savePasswordToExtension(formData);
      });
    }
    
    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => {
        console.log('PassMann: Cancel button clicked');
        removeSavePopup();
      });
    }
    
    // Auto-dismiss after 10 seconds
    setTimeout(() => {
      console.log('PassMann: Auto-dismissing popup after 10 seconds');
      removeSavePopup();
    }, 10000);
  }
  
  // Remove save popup with animation
  function removeSavePopup() {
    if (savePopup) {
      const container = savePopup.querySelector('#PassMann-popup-container');
      if (container) {
        container.style.animation = 'PassMannSlideOut 0.3s ease-in';
        setTimeout(() => {
          if (savePopup && savePopup.parentNode) {
            savePopup.parentNode.removeChild(savePopup);
          }
          savePopup = null;
        }, 300);
      }
    }
  }
  
  // Save password to PassMann extension
  async function savePasswordToExtension(formData) {
    try {
      // Send message to background script to save password
      chrome.runtime.sendMessage({
        action: 'savePassword',
        data: formData
      }, (response) => {
        if (response && response.success) {
          showSuccessMessage('Password saved to PassMann!');
        } else {
          showSuccessMessage('Failed to save password. Please try again.', 'error');
        }
        removeSavePopup();
      });
    } catch (error) {
      console.error('Error saving password:', error);
      showSuccessMessage('Failed to save password. Please try again.', 'error');
      removeSavePopup();
    }
  }
  
  // Show success/error message
  function showSuccessMessage(message, type = 'success') {
    const messageDiv = document.createElement('div');
    messageDiv.style.cssText = `
      position: fixed;
      top: 20px;
      right: 20px;
      z-index: 10002;
      background: ${type === 'success' ? '#4caf50' : '#f44336'};
      color: white;
      padding: 12px 16px;
      border-radius: 4px;
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      font-size: 14px;
      font-weight: 500;
      box-shadow: 0 4px 12px rgba(0,0,0,0.3);
      animation: PassMannSlideIn 0.3s ease-out;
    `;
    messageDiv.textContent = message;
    
    document.body.appendChild(messageDiv);
    
    setTimeout(() => {
      messageDiv.style.animation = 'PassMannSlideOut 0.3s ease-in';
      setTimeout(() => {
        if (messageDiv.parentNode) {
          messageDiv.parentNode.removeChild(messageDiv);
        }
      }, 300);
    }, 3000);
  }
  
  // Monitor form submissions to trigger save popup (Google-style)
  function monitorFormSubmissions() {
    console.log('PassMann: Form monitoring started');
    
    // Monitor form submit events
    document.addEventListener('submit', (event) => {
      console.log('PassMann: Form submit detected', event.target);
      const form = event.target;
      if (form.tagName === 'FORM') {
        // Capture form data immediately before any potential redirect
        const formData = extractFormData();
        console.log('PassMann: Extracted form data:', formData);
        if (formData) {
          // Store the data immediately
          currentFormData = formData;
          // Show popup immediately - don't wait for submission to complete
          createSavePopup(formData);
        }
      }
    });
    
    // Monitor click events on submit buttons and login buttons
    document.addEventListener('click', (event) => {
      const target = event.target;
      const isSubmitButton = target.type === 'submit' || 
                           (target.tagName === 'BUTTON' && target.type !== 'button') ||
                           target.textContent && /sign\s*in|log\s*in|login|submit|continue|next/i.test(target.textContent);
      
      if (isSubmitButton) {
        console.log('PassMann: Submit button clicked', target);
        // Capture form data when login button is clicked
        const formData = extractFormData();
        console.log('PassMann: Form data from button click:', formData);
        if (formData) {
          currentFormData = formData;
          // Small delay to capture any last-minute field changes
          setTimeout(() => {
            createSavePopup(formData);
          }, 100);
        }
      }
    });
    
    // Monitor Enter key presses in password fields
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') {
        const isPasswordField = event.target.type === 'password';
        const isUsernameField = event.target.type === 'email' || 
                               (event.target.type === 'text' && 
                                /user|email/i.test(event.target.name + event.target.id + event.target.placeholder));
        
        if (isPasswordField || isUsernameField) {
          console.log('PassMann: Enter key in form field', event.target);
          const formData = extractFormData();
          console.log('PassMann: Form data from Enter key:', formData);
          if (formData) {
            currentFormData = formData;
            setTimeout(() => {
              createSavePopup(formData);
            }, 50);
          }
        }
      }
    });
    
    // Monitor input changes to capture data in real-time
    document.addEventListener('input', (event) => {
      const target = event.target;
      if (target.type === 'password' || target.type === 'email' || 
          (target.type === 'text' && /user|email/i.test(target.name + target.id + target.placeholder))) {
        // Cache the current form data
        const formData = extractFormData();
        if (formData) {
          currentFormData = formData;
        }
      }
    });
  }

  // Enhanced form field detection
  function findFormFields() {
    const usernameSelectors = [
      'input[type="email"]',
      'input[type="text"][name*="user" i]',
      'input[type="text"][name*="email" i]',
      'input[type="text"][id*="user" i]',
      'input[type="text"][id*="email" i]',
      'input[type="text"][placeholder*="email" i]',
      'input[type="text"][placeholder*="username" i]',
      'input[name="username"]',
      'input[name="email"]',
      'input[id="username"]',
      'input[id="email"]'
    ];
    
    const passwordSelectors = [
      'input[type="password"]'
    ];
    
    let usernameField = null;
    let passwordField = null;
    
    // Find visible username field
    for (const selector of usernameSelectors) {
      const field = document.querySelector(selector);
      if (field && isVisible(field)) {
        usernameField = field;
        break;
      }
    }
    
    // Find visible password field
    for (const selector of passwordSelectors) {
      const field = document.querySelector(selector);
      if (field && isVisible(field)) {
        passwordField = field;
        break;
      }
    }
    
    return { usernameField, passwordField };
  }
  
  function isVisible(element) {
    return element && 
           element.offsetParent !== null && 
           getComputedStyle(element).visibility !== 'hidden' &&
           getComputedStyle(element).display !== 'none';
  }

  // Initialize - Silent monitoring like Google Password Manager
  function init() {
    console.log('PassMann: Initializing content script');
    
    // Set up comprehensive form monitoring
    setupFormMonitoring();
    
    // Check for saved credentials after a short delay to ensure page is loaded
    setTimeout(() => {
      checkForSavedCredentials();
    }, 1000);
    
    console.log('PassMann: Content script initialization complete');
  }
  
  // Initial check and start
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
  
  // Listen for messages from popup
  chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.action === 'fillCredentials') {
      const { usernameField, passwordField } = findFormFields();
      
      if (usernameField) {
        usernameField.value = request.data.username;
        usernameField.dispatchEvent(new Event('input', { bubbles: true }));
        usernameField.dispatchEvent(new Event('change', { bubbles: true }));
      }
      
      if (passwordField) {
        passwordField.value = request.data.password;
        passwordField.dispatchEvent(new Event('input', { bubbles: true }));
        passwordField.dispatchEvent(new Event('change', { bubbles: true }));
      }
      
      sendResponse({ success: true });
    }
    
    if (request.action === 'getPageInfo') {
      sendResponse({
        url: window.location.href,
        title: document.title,
        domain: window.location.hostname
      });
    }
  });

})();
