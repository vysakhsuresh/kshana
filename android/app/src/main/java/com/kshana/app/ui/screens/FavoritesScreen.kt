package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.Expert
import com.kshana.app.data.Favorites
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.glass

/** Pages a new person usually wants close at hand, offered until they star their own. */
private val SUGGESTED = listOf("parihara", "wall", "letter", "journal", "guide")

/**
 * The person's own shelf: the pages they starred, in their order (with arrows to reorder),
 * then the pages they open most that are not starred yet, then a few suggestions.
 */
@Composable
fun FavoritesScreen(onOpen: (String) -> Unit) {
    val context = LocalContext.current
    LaunchedEffect(Unit) { Favorites.reload(context) }
    val expert = Expert.on(context)
    val visible = MORE_PAGES.filter { !it.expert || expert }.associateBy { it.route }
    val pinned = Favorites.pinned.mapNotNull { visible[it] }
    val often = Favorites.opens.entries
        .filter { it.value >= 2 && !Favorites.isPinned(it.key) && it.key in visible }
        .sortedByDescending { it.value }.take(4).mapNotNull { visible[it.key] }
    val open: (String) -> Unit = { Favorites.opened(context, it); onOpen(it) }
    val clock = com.kshana.app.ui.theme.rememberAncientClock()

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 44.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("FAVORITES", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("Kept close", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)

        if (pinned.isEmpty()) {
            Column(Modifier.fillMaxWidth().glass(14.dp).padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("YOUR SHELF IS EMPTY", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
                Text(
                    "Tap the ☆ beside any page here or under Explore, and it stays on this tab, one tap away.",
                    fontFamily = FontFamily.Serif, fontSize = 16.sp, lineHeight = 24.sp, color = KshanaColors.Text,
                )
            }
        } else {
            SectionLabel("Starred")
            PageGrid(pinned, clock, open, footer = { p ->
                val i = pinned.indexOf(p)
                if (pinned.size > 1) {
                    Row(Modifier.fillMaxWidth().padding(end = 8.dp), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
                        if (i > 0) Text("‹ EARLIER", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted,
                            modifier = Modifier.clickable { Favorites.move(context, p.route, -1) }.padding(horizontal = 6.dp, vertical = 6.dp))
                        if (i < pinned.lastIndex) Text("LATER ›", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted,
                            modifier = Modifier.clickable { Favorites.move(context, p.route, 1) }.padding(horizontal = 6.dp, vertical = 6.dp))
                    }
                }
            })
        }

        if (often.isNotEmpty()) {
            SectionLabel("You open these often")
            PageGrid(often, clock, open)
        }

        val suggestions = SUGGESTED.filter { !Favorites.isPinned(it) && often.none { o -> o.route == it } }.mapNotNull { visible[it] }
        if (pinned.size < 3 && suggestions.isNotEmpty()) {
            SectionLabel("Worth keeping close")
            PageGrid(suggestions, clock, open)
        }

        QuietButton("SEE EVERY PAGE UNDER EXPLORE", onClick = { onOpen("more") }, modifier = Modifier.fillMaxWidth().padding(top = 12.dp))
    }
}
