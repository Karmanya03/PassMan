from PIL import Image, ImageDraw
import os

def create_icon(size, filename):
    # Create a new image with RGBA mode (for transparency)
    img = Image.new('RGBA', (size, size), (59, 130, 246, 255))  # Blue background
    draw = ImageDraw.Draw(img)
    
    # Calculate proportions based on size
    scale = size / 24
    
    # Draw lock icon (simplified)
    lock_width = int(10 * scale)
    lock_height = int(8 * scale)
    lock_x = (size - lock_width) // 2
    lock_y = size // 2
    
    # Lock body (rectangle)
    draw.rectangle([lock_x, lock_y, lock_x + lock_width, lock_y + lock_height], 
                   fill=(255, 255, 255, 255))
    
    # Lock shackle (arc approximation with lines)
    shackle_center_x = size // 2
    shackle_center_y = lock_y - int(2 * scale)
    shackle_radius = int(4 * scale)
    
    # Draw shackle as lines
    for angle in range(180, 361, 10):
        x1 = shackle_center_x + int(shackle_radius * cos(radians(angle)))
        y1 = shackle_center_y + int(shackle_radius * sin(radians(angle)))
        x2 = shackle_center_x + int((shackle_radius - 2) * cos(radians(angle)))
        y2 = shackle_center_y + int((shackle_radius - 2) * sin(radians(angle)))
        draw.line([x1, y1, x2, y2], fill=(255, 255, 255, 255), width=max(1, int(scale)))
    
    # Save the image
    img.save(filename, 'PNG')

# Import math functions
from math import cos, sin, radians

# Create icons directory
os.makedirs('extension/icons', exist_ok=True)

# Generate all required icon sizes
sizes = [16, 32, 48, 128]
for size in sizes:
    create_icon(size, f'extension/icons/icon{size}.png')
    print(f'Created icon{size}.png')

print('All icons created successfully!')
