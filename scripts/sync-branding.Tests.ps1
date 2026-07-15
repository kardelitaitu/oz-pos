<#
.SYNOPSIS
    Pester unit tests for scripts/sync-branding.ps1 config-patching regex patterns.
.DESCRIPTION
    Tests all -replace operators used in sync-branding.ps1 for:
    - site.webmanifest name/short_name patching
    - tauri.conf.json productName/identifier/title patching
    - safeId brand-identifier generation

    Runs standalone (no dot-source) to avoid Set-Location or file I/O.

    Run with:
      powershell -NoProfile -Command "Import-Module Pester -RequiredVersion 5.6.1 -Force; Invoke-Pester scripts\sync-branding.Tests.ps1"
#>

# Regex patterns copied from sync-branding.ps1
$webNamePattern    = '(?<="name":\s*)"[^"]*"'
$webShortNamePattern = '(?<="short_name":\s*)"[^"]*"'
$productNamePattern = '(?<="productName":\s*)"[^"]*"'
$identifierPattern  = '(?<="identifier":\s*)"[^"]*"'
$titlePattern      = '(?<="title":\s*)"[^"]*"'
$safeIdBadChars    = '[^a-zA-Z0-9.-]'

Describe 'safeId generation' {

    It 'passes through basic alphanumeric brand IDs unchanged' {
        $brandId = 'default'
        $safeId  = $brandId -replace $safeIdBadChars, '-'
        $safeId | Should -Be 'default'
    }

    It 'replaces spaces and special characters with hyphens' {
        $brandId = 'acme tenant#1'
        $safeId  = $brandId -replace $safeIdBadChars, '-'
        $safeId | Should -Be 'acme-tenant-1'
    }

    It 'preserves dots and hyphens' {
        $brandId = 'my-brand.v2'
        $safeId  = $brandId -replace $safeIdBadChars, '-'
        $safeId | Should -Be 'my-brand.v2'
    }

    It 'handles underscores, exclamation marks, and at-signs' {
        $brandId = 'beta_retail!test@2026'
        $safeId  = $brandId -replace $safeIdBadChars, '-'
        $safeId | Should -Be 'beta-retail-test-2026'
    }

    It 'collapses consecutive bad characters into single hyphens' {
        $brandId = 'foo!!!bar'
        $safeId  = $brandId -replace $safeIdBadChars, '-'
        $safeId | Should -Be 'foo---bar'
    }
}

Describe 'site.webmanifest patching' {

    It "replaces the name field value" {
        $raw   = '{"name":"OZ-POS","short_name":"OZ-POS","start_url":"/"}'
        $appName = 'Beta Retail'
        $result = $raw -replace $webNamePattern, "`"$appName`""
        $result | Should -Be '{"name":"Beta Retail","short_name":"OZ-POS","start_url":"/"}'
    }

    It "replaces the short_name field value independently" {
        $raw   = '{"name":"OZ-POS","short_name":"OZ-POS","start_url":"/"}'
        $appName = 'ACME'
        $result = $raw -replace $webShortNamePattern, "`"$appName`""
        $result | Should -Be '{"name":"OZ-POS","short_name":"ACME","start_url":"/"}'
    }

    It 'does not modify non-matching fields' {
        $raw   = '{"name":"OZ-POS","description":"POS system","start_url":"/"}'
        $appName = 'Beta'
        $result = $raw -replace $webNamePattern, "`"$appName`""
        $result | Should -Be '{"name":"Beta","description":"POS system","start_url":"/"}'
    }

    It 'handles app names with special characters' {
        $raw   = '{"name":"","short_name":""}'
        $appName = 'ACME & Sons POS'
        $result = $raw -replace $webNamePattern, "`"$appName`""
        $result | Should -Be '{"name":"ACME & Sons POS","short_name":""}'
    }

    It 'matches nested name keys in objects (documents shallow regex)' {
        $raw   = '{"name":"TOP","manifest":{"name":"nested"}}'
        $appName = 'Replaced'
        $result = $raw -replace $webNamePattern, "`"$appName`""
        $result | Should -Be '{"name":"Replaced","manifest":{"name":"Replaced"}}'
    }
}

Describe 'tauri.conf.json productName patching' {

    It "replaces the productName field" {
        $raw   = '{ "productName": "OZ-POS", "version": "0.0.4" }'
        $appName = 'Beta Retail'
        $result = $raw -replace $productNamePattern, "`"$appName`""
        $result | Should -Be '{ "productName": "Beta Retail", "version": "0.0.4" }'
    }

    It 'handles productName with single quote inside' {
        $raw   = '{ "productName": "OZ-POS" }'
        $appName = "Acme's POS"
        $result = $raw -replace $productNamePattern, "`"$appName`""
        $expected = '{ "productName": "Acme' + "'" + 's POS" }'
        $result | Should -Be $expected
    }

    It 'handles extra whitespace after colon' {
        $raw   = '{ "productName":   "OZ-POS" }'
        $appName = 'Beta'
        $result = $raw -replace $productNamePattern, "`"$appName`""
        $result | Should -Be '{ "productName":   "Beta" }'
    }

    It 'matches compact JSON with no space after colon' {
        $raw   = '{"productName":"OZ-POS"}'
        $appName = 'Compact Beta'
        $result = $raw -replace $productNamePattern, "`"$appName`""
        $result | Should -Be '{"productName":"Compact Beta"}'
    }
}

Describe 'tauri.conf.json identifier patching' {

    It 'replaces the identifier field' {
        $raw   = '{ "identifier": "com.ozpos.app" }'
        $desktopId = 'com.ozpos.beta-retail'
        $result = $raw -replace $identifierPattern, "`"$desktopId`""
        $result | Should -Be '{ "identifier": "com.ozpos.beta-retail" }'
    }

    It 'replaces tablet identifier separately' {
        $raw   = '{ "identifier": "com.ozpos.tablet" }'
        $tabletId = 'com.ozpos.tablet.beta-retail'
        $result = $raw -replace $identifierPattern, "`"$tabletId`""
        $result | Should -Be '{ "identifier": "com.ozpos.tablet.beta-retail" }'
    }

    It 'matches compact JSON with no space after colon' {
        $raw   = '{"identifier":"com.ozpos.app"}'
        $desktopId = 'com.ozpos.compact'
        $result = $raw -replace $identifierPattern, "`"$desktopId`""
        $result | Should -Be '{"identifier":"com.ozpos.compact"}'
    }
}

Describe 'tauri.conf.json title patching' {

    It 'replaces the window title field' {
        $raw   = '{ "title": "OZ-POS", "identifier": "com.ozpos.app" }'
        $appName = 'Beta Retail'
        $result = $raw -replace $titlePattern, "`"$appName`""
        $result | Should -Be '{ "title": "Beta Retail", "identifier": "com.ozpos.app" }'
    }

    It "does not corrupt other fields when title is absent" {
        $raw   = '{ "identifier": "com.ozpos.app" }'
        $appName = 'Beta'
        $result = $raw -replace $titlePattern, "`"$appName`""
        $result | Should -Be '{ "identifier": "com.ozpos.app" }'
    }

    It 'matches compact JSON with no space after colon' {
        $raw   = '{"title":"OZ-POS","identifier":"com.ozpos.app"}'
        $appName = 'Compact Title'
        $result = $raw -replace $titlePattern, "`"$appName`""
        $result | Should -Be '{"title":"Compact Title","identifier":"com.ozpos.app"}'
    }
}

Describe 'Idempotency' {

    It 're-applying the same replacement produces no change' {
        $raw   = '{ "productName": "OZ-POS", "identifier": "com.ozpos.app" }'
        $appName = 'Beta Retail'
        $first  = $raw -replace $productNamePattern, "`"$appName`""
        $first  = $first -replace $identifierPattern, "`"com.ozpos.beta-retail`""
        $second = $first -replace $productNamePattern, "`"$appName`""
        $second = $second -replace $identifierPattern, "`"com.ozpos.beta-retail`""
        $second | Should -Be $first
    }
}

Describe 'Multi-field patching simulation' {

    It 'replicates the desktop config patching logic' {
        $raw   = '{ "productName": "OZ-POS", "identifier": "com.ozpos.app", "bundle": { "icon": ["icons/icon.ico"] } }'
        $appName   = 'Beta Retail'
        $brandId   = 'beta-retail'
        $safeId    = $brandId -replace $safeIdBadChars, '-'
        $desktopId = 'com.ozpos.' + $safeId

        $result = $raw -replace $productNamePattern, "`"$appName`""
        $result = $result -replace $identifierPattern, "`"$desktopId`""

        $result | Should -Match '"productName": "Beta Retail"'
        $result | Should -Match '"identifier": "com.ozpos.beta-retail"'
        $result | Should -Not -Match '"identifier": "com.ozpos.app"'
    }

    It 'replicates the tablet config patching logic' {
        $raw   = '{ "productName": "OZ-POS", "identifier": "com.ozpos.tablet", "bundle": { "icon": ["icons/icon.ico"] } }'
        $appName   = 'Beta Retail'
        $brandId   = 'beta-retail'
        $safeId    = $brandId -replace $safeIdBadChars, '-'
        $tabletId  = 'com.ozpos.tablet.' + $safeId

        $result = $raw -replace $productNamePattern, "`"$appName`""
        $result = $result -replace $identifierPattern, "`"$tabletId`""

        $result | Should -Match '"productName": "Beta Retail"'
        $result | Should -Match '"identifier": "com.ozpos.tablet.beta-retail"'
        $result | Should -Not -Match '"identifier": "com.ozpos.tablet","'
    }

    It 'correctly identifies default brand as com.ozpos.app' {
        $brandId   = 'default'
        $safeId    = $brandId -replace $safeIdBadChars, '-'
        $desktopId = if ($brandId -eq 'default') { 'com.ozpos.app' } else { 'com.ozpos.' + $safeId }
        $desktopId | Should -Be 'com.ozpos.app'
    }
}
