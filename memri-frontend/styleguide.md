# Eden.so Complete UI/UX Design Specification
## Design System & Theming Prompt for AI Implementation

---

## Color Palette & Visual Foundation

### Primary Colors
- **Background (Base)**: Pure white (#FFFFFF) for light mode
- **Background (Dark)**: Deep charcoal/near-black (#0A0A0A to #121212) for dark mode contexts
- **Primary Accent**: Vibrant cyan/turquoise (#00E5CC to #00D4B8) - used sparingly for CTAs and highlights
- **Secondary Accent**: Soft purple/lavender (#8B7CFF to #9B8AFF) - used for secondary interactions

### Text Hierarchy
- **Primary Text**: Pure black (#000000) on light backgrounds
- **Secondary Text**: Medium gray (#666666 to #707070) for supporting information
- **Tertiary Text**: Light gray (#999999 to #AAAAAA) for timestamps and metadata
- **Inverse Text**: White (#FFFFFF) on dark surfaces

### UI Surface Colors
- **Card Background**: White (#FFFFFF) with subtle elevation shadows
- **Border Color**: Very light gray (#E5E5E5 to #EEEEEE) - 1px borders
- **Hover State**: Extremely subtle gray tint (#FAFAFA to #F8F8F8)
- **Active/Selected State**: Light cyan tint (#E6FFFE to #F0FFFD)
- **Focus Ring**: Cyan accent with 2px solid outline

---

## Typography System

### Font Family
- **Primary**: Modern sans-serif, likely Inter, SF Pro, or similar geometric sans
- **Weight Range**: 400 (Regular), 500 (Medium), 600 (Semibold), 700 (Bold)

### Font Sizes & Line Heights
- **Display Large**: 48-56px / Line height 1.1 / Weight 700
- **Heading 1**: 32-36px / Line height 1.2 / Weight 700
- **Heading 2**: 24-28px / Line height 1.3 / Weight 600
- **Heading 3**: 18-20px / Line height 1.4 / Weight 600
- **Body Large**: 16px / Line height 1.5 / Weight 400
- **Body Regular**: 14px / Line height 1.5 / Weight 400
- **Body Small**: 12px / Line height 1.4 / Weight 400
- **Caption**: 11px / Line height 1.3 / Weight 500

### Letter Spacing
- **Display/Headings**: -0.02em (tight)
- **Body Text**: 0em (normal)
- **Uppercase Labels**: 0.05em (loose)

---

## Chat Interface Components

### Message Input Bar

#### Container Specifications
- **Position**: Fixed at bottom of chat area or canvas workspace
- **Width**: 100% of container with 16-24px horizontal padding
- **Min Height**: 52px (single line)
- **Max Height**: 200px (expands with multiline input)
- **Background**: White (#FFFFFF)
- **Border**: 
  - Top: 1px solid #E5E5E5
  - All sides: 1px solid #E5E5E5 when floating
- **Border Radius**: 12px (for floating variant) or 0px (for docked variant)
- **Box Shadow**: 
  - Docked: none or subtle top shadow `0 -1px 3px rgba(0,0,0,0.04)`
  - Floating: `0 2px 12px rgba(0,0,0,0.08)`

#### Input Field
- **Font Size**: 14px
- **Font Weight**: 400
- **Line Height**: 20px
- **Padding**: 14px 48px 14px 16px (to accommodate send button)
- **Color**: #000000
- **Placeholder Color**: #999999
- **Placeholder Text**: "Type a message..." or "Ask anything..."
- **Resize Behavior**: Smooth auto-expansion vertically
- **Focus State**: 
  - Border changes to cyan accent (#00D4B8) 2px
  - Subtle glow: `0 0 0 3px rgba(0, 213, 184, 0.1)`
- **Caret Color**: Cyan accent (#00D4B8)

#### Send Button
- **Position**: Absolute right, vertically centered
- **Size**: 32px × 32px
- **Right Margin**: 10px from container edge
- **Icon**: Paper airplane or arrow right (16px)
- **Default State**:
  - Background: Transparent or very light gray (#F5F5F5)
  - Icon Color: Medium gray (#999999)
  - Border Radius: 8px
- **Hover State**:
  - Background: Light cyan (#E6FFFE)
  - Icon Color: Cyan accent (#00D4B8)
  - Smooth transition: 150ms ease
- **Active State**:
  - Background: Cyan accent (#00D4B8)
  - Icon Color: White (#FFFFFF)
- **Disabled State**:
  - Background: Transparent
  - Icon Color: Extra light gray (#CCCCCC)
  - Cursor: not-allowed

#### Additional Input Controls
- **Attachment Button** (if present):
  - Position: Left side, 8px from edge
  - Size: 28px × 28px
  - Icon: Paperclip or plus (14px)
  - Color: Medium gray (#707070)
  - Hover: Cyan accent (#00D4B8)

- **Voice Input** (optional):
  - Position: Adjacent to attachment button
  - Size: 28px × 28px
  - Icon: Microphone (14px)
  - Recording state: Pulsing red dot animation

---

## Chat Message Display

### Message Bubbles

#### User Messages (Outgoing)
- **Alignment**: Right-aligned with 12-16px right margin
- **Max Width**: 70% of container
- **Background**: Linear gradient from light cyan to light purple (#E6FFFE to #F0EFFF) or solid cyan tint
- **Text Color**: Black (#000000) or very dark gray (#1A1A1A)
- **Padding**: 12px 16px
- **Border Radius**: 16px 16px 4px 16px (rounded except bottom-right)
- **Font Size**: 14px
- **Line Height**: 1.5
- **Box Shadow**: `0 1px 2px rgba(0,0,0,0.04)`
- **Margin Bottom**: 8px between consecutive messages, 16px between different senders

#### AI/System Messages (Incoming)
- **Alignment**: Left-aligned with 12-16px left margin
- **Max Width**: 70% of container
- **Background**: White (#FFFFFF) or very light gray (#F8F8F8)
- **Text Color**: Black (#000000)
- **Border**: 1px solid #E5E5E5
- **Padding**: 12px 16px
- **Border Radius**: 16px 16px 16px 4px (rounded except bottom-left)
- **Font Size**: 14px
- **Line Height**: 1.5
- **Box Shadow**: `0 1px 2px rgba(0,0,0,0.04)`

#### Message Metadata
- **Timestamp**: 
  - Font Size: 11px
  - Color: #999999
  - Position: Below message, right-aligned for user, left-aligned for AI
  - Margin Top: 4px
- **Status Indicators** (if applicable):
  - Size: 12px × 12px
  - Position: Adjacent to timestamp
  - Colors: Gray (sending), cyan (sent), green (read)

---

## Cursor & Interaction States

### Text Cursor
- **Default Cursor**: System default I-beam in input fields
- **Caret**: 
  - Color: Cyan accent (#00D4B8)
  - Width: 2px
  - Animation: 1s blink cycle

### UI Element Cursors
- **Clickable Elements**: `cursor: pointer`
- **Draggable Elements**: `cursor: grab` (idle) / `cursor: grabbing` (active)
- **Resizable Elements**: Appropriate resize cursors (ew-resize, ns-resize)
- **Disabled Elements**: `cursor: not-allowed`

### Loading States
- **Typing Indicator** (when AI is responding):
  - Container: Same as AI message bubble
  - Animation: Three animated dots
  - Dot Size: 6px diameter
  - Dot Color: Medium gray (#999999)
  - Animation: Gentle bounce, 1.4s duration, staggered by 0.2s
  - Spacing: 4px between dots

---

## Spacing & Layout System

### Container Padding
- **Mobile**: 16px horizontal, 12px vertical
- **Tablet**: 24px horizontal, 16px vertical
- **Desktop**: 32px horizontal, 24px vertical

### Component Spacing
- **Tight**: 4px - Between related inline elements
- **Close**: 8px - Between related block elements
- **Default**: 16px - Between unrelated elements
- **Comfortable**: 24px - Between sections
- **Spacious**: 32px - Between major sections

### Message List
- **Padding**: 16px horizontal
- **Gap Between Messages**: 
  - Same sender: 4px
  - Different sender: 16px
- **Scroll Behavior**: Smooth scroll, auto-scroll to bottom on new message
- **Scroll Bar**: 
  - Width: 6px
  - Track: Transparent or #F5F5F5
  - Thumb: #CCCCCC
  - Thumb (hover): #999999

---

## Animation & Transitions

### Standard Transitions
- **Property**: `all`
- **Duration**: 150ms (fast) or 250ms (standard)
- **Easing**: `cubic-bezier(0.4, 0.0, 0.2, 1)` (ease-out)

### Message Animations
- **New Message Entry**:
  - Animation: Fade in + slide up 12px
  - Duration: 300ms
  - Easing: `cubic-bezier(0.0, 0.0, 0.2, 1)`

- **Message Sending**:
  - Animation: Fade in with slight scale (0.95 to 1.0)
  - Duration: 200ms
  
- **Hover Effects**:
  - Duration: 150ms
  - Properties: background-color, transform (slight scale or elevation)

### Focus Animations
- **Focus Ring Growth**: 200ms ease-out
- **Input Expansion**: 250ms ease-in-out (for multiline growth)

---

## Accessibility Specifications

### Focus Indicators
- **Keyboard Focus**: 
  - Visible 2px solid cyan outline (#00D4B8)
  - Offset: 2px from element
  - Border Radius: Match element + 2px

### Contrast Ratios
- **Normal Text**: Minimum 4.5:1 (WCAG AA)
- **Large Text**: Minimum 3:1
- **UI Components**: Minimum 3:1
- **All combinations tested**: Cyan on white, black on white, gray text on backgrounds

### Screen Reader Support
- **Input Label**: Proper aria-label "Message input field"
- **Send Button**: aria-label "Send message"
- **Message List**: role="log" aria-live="polite"
- **Typing Indicator**: aria-label "AI is typing"

---

## Responsive Behavior

### Breakpoints
- **Mobile**: 0-767px
  - Input height: 48px min
  - Message width: 85% max
  - Font size: 14px
  
- **Tablet**: 768-1023px
  - Input height: 52px min
  - Message width: 75% max
  - Font size: 14px

- **Desktop**: 1024px+
  - Input height: 52px min
  - Message width: 70% max
  - Font size: 14px

### Touch Targets
- **Minimum Size**: 44px × 44px (iOS) or 48px × 48px (Android)
- **Button Spacing**: Minimum 8px between touch targets

---

## Special States & Features

### File Attachments (in messages)
- **Preview**: 
  - Max Width: 280px
  - Border Radius: 8px
  - Margin: 8px 0
  - Box Shadow: `0 1px 3px rgba(0,0,0,0.1)`

### Code Blocks (in messages)
- **Background**: #F5F5F5 (light) or #1E1E1E (dark)
- **Font**: Monospace (Monaco, Consolas, Courier New)
- **Font Size**: 13px
- **Padding**: 12px
- **Border Radius**: 6px
- **Syntax Highlighting**: Subtle colors matching theme

### Links in Messages
- **Color**: Cyan accent (#00D4B8)
- **Hover**: Underline
- **Visited**: Slightly darker cyan (#00B8A3)

---

## Brand-Specific Design Elements

### Minimalist Philosophy
- **Cleanliness**: Ample white space, minimal visual noise
- **Hierarchy**: Clear through size and weight, not decoration
- **Functional**: Every element serves a purpose
- **Modern**: Contemporary, not trendy or dated

### Visual Restraint
- **Shadows**: Subtle, used for depth only
- **Borders**: 1px, light gray, used sparingly
- **Colors**: Limited palette, strategic accent use
- **Effects**: Minimal gradients, no heavy textures

### Micro-interactions
- **Button Press**: Subtle scale down (0.97) on click
- **Card Hover**: Minimal elevation increase (shadow intensifies)
- **Input Focus**: Smooth border color transition
- **Icon Rotation**: 180° for toggles, 200ms duration

---

## Implementation Notes

### CSS Custom Properties (Variables)
```css
:root {
  /* Colors */
  --color-primary: #00D4B8;
  --color-secondary: #9B8AFF;
  --color-bg: #FFFFFF;
  --color-text: #000000;
  --color-text-secondary: #707070;
  --color-text-tertiary: #999999;
  --color-border: #E5E5E5;
  --color-hover: #F8F8F8;
  
  /* Spacing */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;
  
  /* Border Radius */
  --radius-sm: 6px;
  --radius-md: 12px;
  --radius-lg: 16px;
  
  /* Transitions */
  --transition-fast: 150ms cubic-bezier(0.4, 0.0, 0.2, 1);
  --transition-standard: 250ms cubic-bezier(0.4, 0.0, 0.2, 1);
}
```

### Z-Index Layering
- **Base Content**: 1
- **Sticky Headers**: 10
- **Dropdowns/Popovers**: 100
- **Modals**: 1000
- **Toasts/Notifications**: 2000
- **Tooltips**: 3000

---

## Summary Statement

Eden.so employs a sophisticated yet minimalist design language characterized by generous white space, subtle color accents, and purposeful typography. The interface prioritizes clarity and functionality over ornamentation, with cyan and purple accents used strategically to guide user attention. Every interaction is smooth and considered, with micro-animations that feel natural rather than showy. The chat interface should feel like a natural ext
ension of this design system—clean, responsive, and focused on enabling creativity without visual distraction. The overall aesthetic is contemporary professional with a slight creative edge, suitable for designers, creators, and knowledge workers who value both beauty and utility in their tools.