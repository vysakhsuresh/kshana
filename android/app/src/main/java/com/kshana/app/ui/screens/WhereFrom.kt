package com.kshana.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.kshana.app.data.Lang
import com.kshana.app.ui.theme.KshanaColors
import uniffi.kshana_ffi.SourceRecord

/** Resolved sources, by language and keys: the registry never changes while the app runs. */
private object SourceCache {
    val lines = HashMap<String, List<SourceRecord>>()

    fun of(keys: List<String>, lang: String): List<SourceRecord> {
        val k = lang + "|" + keys.joinToString(",")
        return lines.getOrPut(k) { runCatching { uniffi.kshana_ffi.sourceLines(keys, lang) }.getOrDefault(emptyList()) }
    }
}

/**
 * "Where this comes from": a quiet gold link under a claim or a remedy that opens to the
 * books, chapters and customs behind it, said honestly: a classical text with its
 * chapter, a later folk text, a temple or household custom ("custom, not a text"), or
 * Kshana's own method with what it does. `keys` are a record's `sourceKeys` (or the
 * registry's aliases, such as `life.dasha`). Shows nothing when no key resolves.
 */
@Composable
internal fun WhereFrom(keys: List<String>, modifier: Modifier = Modifier) {
    if (keys.isEmpty()) return
    val lang = Lang.code
    var open by rememberSaveable(keys) { mutableStateOf(false) }
    val sources = remember(keys, lang, open) { if (open) SourceCache.of(keys, lang) else emptyList() }
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(
            if (open) tr("WHERE THIS COMES FROM ‹") else tr("WHERE THIS COMES FROM ›"),
            style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold,
            modifier = Modifier.clickable { open = !open }.padding(vertical = 4.dp),
        )
        AnimatedVisibility(open, enter = fadeIn() + expandVertically()) {
            Column(Modifier.padding(start = 2.dp, bottom = 4.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (sources.isEmpty()) {
                    Text(tr("No source is recorded for this yet."), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
                }
                for (s in sources) SourceLine(s)
            }
        }
    }
}

@Composable
private fun SourceLine(s: SourceRecord) {
    Row {
        Text("·", style = MaterialTheme.typography.bodySmall, color = if (s.kind == "text") KshanaColors.Gold else KshanaColors.Muted)
        Column(Modifier.padding(start = 8.dp).weight(1f)) {
            Text(s.line, style = MaterialTheme.typography.bodySmall, color = if (s.kind == "kshana") KshanaColors.Sky else KshanaColors.TextSoft)
            if (s.kind != "kshana") Text(s.note, style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
        }
    }
}
