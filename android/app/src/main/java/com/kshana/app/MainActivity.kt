package com.kshana.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Star
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.kshana.app.ui.screens.MomentsScreen
import com.kshana.app.ui.screens.NowScreen
import com.kshana.app.ui.theme.KshanaColors
import com.kshana.app.ui.theme.KshanaTheme

private data class Tab(val route: String, val label: String, val icon: ImageVector)

private val tabs = listOf(
    Tab("now", "NOW", Icons.Filled.Star),
    Tab("moments", "MOMENTS", Icons.Filled.DateRange),
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { KshanaTheme { App() } }
    }
}

@Composable
private fun App() {
    val nav = rememberNavController()
    val backStack by nav.currentBackStackEntryAsState()
    val current = backStack?.destination?.route ?: "now"
    Scaffold(
        containerColor = KshanaColors.Night,
        bottomBar = {
            NavigationBar(containerColor = KshanaColors.Night) {
                for (t in tabs) {
                    NavigationBarItem(
                        selected = current == t.route,
                        onClick = { if (current != t.route) nav.navigate(t.route) { launchSingleTop = true; popUpTo("now") } },
                        icon = { Icon(t.icon, contentDescription = t.label) },
                        label = { Text(t.label) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = KshanaColors.Gold,
                            selectedTextColor = KshanaColors.Gold,
                            unselectedIconColor = KshanaColors.Muted,
                            unselectedTextColor = KshanaColors.Muted,
                            indicatorColor = KshanaColors.Surface,
                        ),
                    )
                }
            }
        },
    ) { padding ->
        NavHost(nav, startDestination = "now", modifier = Modifier.padding(padding)) {
            composable("now") { NowScreen() }
            composable("moments") { MomentsScreen() }
        }
    }
}
