## Core palette

| Role             |       Hex |           RGB | Notes                                |
| ---------------- | --------: | ------------: | ------------------------------------ |
| Syntax Green     | `#98C379` | `152, 195, 121`| Main wordmark / success state        |
| Peach Hazard     | `#D19A66` | `209, 154, 102`| Subtle UI strip / warning tone       |
| Coral Red        | `#E06C75` | `224, 108, 117`| Borders, urgency, errors             |
| Lavender Violet  | `#C678DD` | `198, 120, 221`| Side blocks / calm energy accent     |
| Glacier Blue     | `#61AFEF` |  `97, 175, 239`| Info, secondary accents, links       |
| Void Deep        | `#282C34` |   `40, 44, 52` | Main background / dark anchor        |
| Ash White        | `#ABB2BF` | `171, 178, 191`| Primary text / readable foreground   |

## Optional micro-accent

| Role          |       Hex |           RGB | Notes                                |
| ------------- | --------: | ------------: | ------------------------------------ |
| Chalk Yellow  | `#E5C07B` | `229, 192, 123`| Tiny highlights / modified states    |
| Surface Grey  | `#3E4451` |   `62, 68, 81` | Hover states / subtle dividers       |

## Best-use version
* **Base (Background):** `#282C34`
* **Text:** `#ABB2BF`
* **Primary:** `#98C379`
* **Secondary:** `#61AFEF`
* **Accent 1:** `#C678DD`
* **Accent 2:** `#E06C75`

## CSS variables

```css
:root {
  /* Base & Text */
  --void-deep: #282C34;
  --surface-grey: #3E4451;
  --ash-white: #ABB2BF;

  /* Accents */
  --syntax-green: #98C379;
  --peach-hazard: #D19A66;
  --coral-red: #E06C75;
  --lavender-violet: #C678DD;
  --glacier-blue: #61AFEF;
  --chalk-yellow: #E5C07B;
}
```
