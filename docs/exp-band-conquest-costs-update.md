# Updating Exp Band Conquest Point Costs

This guide walks through updating the conquest point costs for experience point bands using xi-tinkerer, with Harara (Windurst Woods) as the example. The workflow: export DATs to YAML, edit the YAML, then convert back to DAT for use in xipivot or your FFXI folder.

---

## 1. Set Up xi-tinkerer

1. Set the **FFXI folder** (path to your FINAL FANTASY XI install).
2. Set the **project folder** — this is where exports will be written and where you'll place modified YAML for conversion.

---

## 2. Export from DAT

1. Go to the **By zone** section in the sidebar.
2. Click **Event**.
3. In the zone list, search for the zone you want (e.g. **Windurst Woods**).
4. Click **Export from DAT** for that zone.

This produces YAML files in your project folder under `exports/raw_data/`:
- `event/Windurst_Woods.yml` — event scripts (Harara's conquest menu)
- `dialog/Windurst_Woods.yml` — dialog strings (recharge messages)

---

## 3. Where the Costs Live

Exp band costs appear in **two files**:

| Use case | File | What to edit |
|----------|------|--------------|
| **Recharging** | `dialog/Windurst_Woods.yml` | Message IDs 9018, 9019, 9020 — "One charge requires 50/100/200 conquest points" |
| **Purchasing** (Chariot/Empress/Emperor) | `event/Windurst_Woods.yml` | Harara block's `immed_data` — values 500, 1000, 2000 |

---

## 4. Modify the YAML

### Recharge costs (50/100/200)

1. Open `exports/raw_data/dialog/Windurst_Woods.yml`.
2. Search for `One charge requires`.
3. Edit the numbers in messages 9018, 9019, 9020 to your new costs.

### Purchase costs (500/1000/2000, Chariot/Empress/Emperor)

1. Open `exports/raw_data/event/Windurst_Woods.yml`.
2. Search for `actor_number: 17764543` (Harara's block).
3. In that block's `immed_data`, search for `500`, `1000`, `2000`.
4. Replace those values with your new costs.

---

## 5. Make DAT and Replace

1. Back in xi-tinkerer, go to **By zone**.
2. For each file type you edited:
   - **Event** → search for the zone, click **Make DAT**
   - **Dialog** → search for the zone, click **Make DAT**
3. Replace the generated DAT(s) in the relevant section of your FFXI folder or xipivot (e.g. ROM folder for the zone's event and dialog DATs).

---
