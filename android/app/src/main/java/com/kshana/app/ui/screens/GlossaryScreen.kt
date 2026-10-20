package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.engine.EngineProvider
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.GlossaryRecord
import uniffi.kshana_ffi.glossary
import com.kshana.app.ui.sky.KshanaFonts

private val TRADITIONS = listOf("All", "Vedic", "Western", "Both", "Engine")

/** Every word the app uses, explained by the engine itself. Searchable, filterable by tradition. */
@Composable
fun GlossaryScreen(onBack: () -> Unit, initialTerm: String? = null) {
    val context = LocalContext.current
    var entries by remember { mutableStateOf<List<GlossaryRecord>>(emptyList()) }
    var query by remember { mutableStateOf(initialTerm ?: "") }
    var tradition by remember { mutableStateOf("All") }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        try {
            EngineProvider.get(context)
            entries = withContext(Dispatchers.Default) { glossary() }
            error = null
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }

    val shown = remember(entries, query, tradition) {
        val q = query.trim()
        entries.filter { e ->
            (tradition == "All" || e.tradition.equals(tradition, ignoreCase = true)) &&
                (q.isEmpty() || e.term.contains(q, ignoreCase = true) || e.meaning.contains(q, ignoreCase = true))
        }
    }

    Column(Modifier.fillMaxSize().padding(top = 48.dp)) {
        Column(Modifier.padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                Text("GLOSSARY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
            }
            Text("Every word, explained", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("Search a term or a meaning", style = MaterialTheme.typography.labelSmall) },
                singleLine = true,
                textStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 15.sp, color = KshanaColors.Text),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedTextColor = KshanaColors.Text,
                    unfocusedTextColor = KshanaColors.Text,
                    cursorColor = KshanaColors.Gold,
                    focusedBorderColor = KshanaColors.Gold,
                    unfocusedBorderColor = KshanaColors.Line,
                    focusedLabelColor = KshanaColors.Gold,
                    unfocusedLabelColor = KshanaColors.Muted,
                    focusedContainerColor = KshanaColors.Surface,
                    unfocusedContainerColor = KshanaColors.Surface,
                ),
                modifier = Modifier.fillMaxWidth(),
            )
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (t in TRADITIONS) Chip(t.uppercase(), selected = tradition == t, onClick = { tradition = t })
            }
            ErrorLine(error)
            Text(
                if (entries.isEmpty() && error == null) "loading…" else "${shown.size} of ${entries.size} entries",
                style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted,
            )
        }
        LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = 24.dp, end = 24.dp, top = 6.dp, bottom = 32.dp)) {
            items(shown, key = { it.term + "|" + it.tradition }) { e ->
                HorizontalDivider(color = KshanaColors.LineSoft)
                Column(Modifier.fillMaxWidth().padding(vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(e.term, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
                    Text(e.tradition.uppercase(), style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted)
                    Text(e.meaning, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)
                }
            }
            if (shown.isEmpty() && entries.isNotEmpty()) {
                item {
                    Text("Nothing matches. Try a shorter word.", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted, modifier = Modifier.padding(vertical = 12.dp))
                }
            }
        }
    }
}
