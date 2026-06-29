--[[
  validate_order.lua — Example order validation rule.
  
  Returns a list of error strings. An empty list means the order is valid.
  Each non-empty entry is shown to the cashier before completion.
--]]

function validate_order(lines, total_minor, currency)
    local errors = {}
    local seen_skus = {}

    for i = 1, #lines do
        local line = lines[i]

        -- Reject quantities > 50 (likely keying mistake)
        if line.qty > 50 then
            table.insert(errors, line.sku .. ": quantity " .. line.qty .. " exceeds maximum of 50")
        end

        -- Flag alcohol sales for age verification
        if line.sku:match("^BEER") or line.sku:match("^WINE") or line.sku:match("^SPIRITS") then
            table.insert(errors, line.sku .. ": verify customer age (21+)")
        end

        -- Check for duplicate SKUs (should be merged)
        if seen_skus[line.sku] then
            table.insert(errors, line.sku .. ": duplicate line (merge items)")
        end
        seen_skus[line.sku] = true
    end

    return errors
end
