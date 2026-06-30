--[[
  tax_cigarettes.lua — Example tax override rule.
  
  Applies a higher tax rate to tobacco products and a reduced rate
  to essential grocery items, overriding the default database rate.
  
  Return nil to fall through to the configured database rate.
--]]

function calc_line_tax(sku, qty, unit_price_minor, currency)
    -- Cigarettes: 20% excise tax, inclusive of displayed price
    if sku:match("^CIG") or sku:match("^TOB") then
        return { rate_bps = 2000, is_inclusive = true }
    end

    -- Essential groceries: 0% VAT
    if sku:match("^MILK") or sku:match("^BREAD") or sku:match("^RICE") then
        return { rate_bps = 0, is_inclusive = false }
    end

    -- Prepared food (restaurant mode): 8% GST, exclusive
    if sku:match("^FOOD%-") then
        return { rate_bps = 800, is_inclusive = false }
    end

    -- Fall through to configured DB rate
    return nil
end
