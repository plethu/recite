@{
    IncludeDefaultRules = $true
    Rules = @{
        PSUseConsistentIndentation = @{ Enable = $true; IndentationSize = 4; Kind = 'space' }
        PSUseConsistentWhitespace = @{ Enable = $true }
        PSPlaceOpenBrace = @{ Enable = $true; OnSameLine = $true }
        PSPlaceCloseBrace = @{ Enable = $true; NewLineAfter = $true }
    }
}
