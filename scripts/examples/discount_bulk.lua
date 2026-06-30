--[[
  bulk_discount.lua — Example "buy more, save more" discount rule.
  
  When the total line-item value exceeds a threshold, the script
  returns a percentage discount and a label shown on the receipt.
  
  Return nil (no discount) when the threshold is not met.
--]]

function apply_discount(lines)
    local total_minor = 0
    local item_count = 0
    for i = 1, #lines do
        total_minor = total_minor + lines[i].qty * lines[i].unit_price_minor
        item_count = item_count + lines[i].qty
    end

    -- Tier 1: 10+ items → 10% off
    if item_count >= 10 then
        return { percent = 10, label = "Bulk 10+" }
    end

    -- Tier 2: total exceeds $50.00 (5000 minor units) → 5% off
    if total_minor > 5000 then
        return { percent = 5, label = "Volume" }
    end

    return nil
end
