# 🎬 **How to Add Your Custom GIF Background**

## 📁 **File Location:**
```
PassMann/extension/background.gif
```

## 🔧 **Steps to Replace:**

1. **Get Your GIF:**
   - Find or create the GIF you want to use
   - Make sure it's in `.gif` format

2. **Rename Your GIF:**
   - Rename your GIF file to exactly: `background.gif`

3. **Replace the File:**
   - Navigate to: `PassMann/extension/`
   - Delete the existing `background.gif` (placeholder)
   - Copy your renamed GIF file to this location

4. **Reload Extension:**
   - Go to `chrome://extensions/`
   - Find PassMann extension
   - Click the reload button 🔄
   - Open the extension to see your new background!

## ✅ **Current Setup:**

- ✅ **Theme selector removed** - no more buttons
- ✅ **Single GIF background** - clean and simple
- ✅ **Easy replacement** - just replace one file
- ✅ **Perfect visibility** - UI adapts to any GIF

## 🎯 **File Info:**

- **Exact file name required:** `background.gif`
- **Location:** `PassMann/extension/background.gif`
- **Format:** GIF (animated or static)
- **Recommended size:** 380x600px or larger
- **Max recommended file size:** 5MB

## 🎨 **GIF Recommendations:**

### **✅ Works Best:**
- Dark or medium-toned GIFs
- Subtle animations
- Good contrast backgrounds
- Abstract patterns or textures

### **⚠️ Might Need Adjustment:**
- Very bright or white GIFs
- High-contrast flashing animations
- Very busy/chaotic patterns

### **💡 If Text is Hard to Read:**
You can adjust the overlay opacity in `popup.css` line 60-70:
```css
body::before {
  opacity: 0.9; /* Increase for darker overlay */
}
```

## 🚀 **That's It!**

Your extension now uses a single, easily replaceable GIF background. Just replace the `background.gif` file with your own and reload the extension!
