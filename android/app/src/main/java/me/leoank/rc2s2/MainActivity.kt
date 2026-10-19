package me.leoank.rc2s2

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewmodel.compose.viewModel
import uniffi.rc2s2_core.NodeInfo
import uniffi.rc2s2_core.SessionInfo

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MaterialTheme(colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme()) {
                App()
            }
        }
    }
}

/** Where the person is; a small stack in place of a navigation library. */
sealed interface Screen {
    data object Home : Screen
    data class Session(val session: SessionInfo) : Screen
    data class Health(val node: NodeInfo) : Screen
}

@Composable
fun App(model: HubModel = viewModel()) {
    var screen by remember { mutableStateOf<Screen>(Screen.Home) }
    if (!model.connected) {
        ConnectScreen(model)
        return
    }
    BackHandler(enabled = screen != Screen.Home) {
        model.close()
        screen = Screen.Home
    }
    when (val s = screen) {
        Screen.Home -> HomeScreen(
            model,
            openSession = {
                model.open(it)
                screen = Screen.Session(it)
            },
            openHealth = {
                model.loadHealth(it.id)
                screen = Screen.Health(it)
            },
        )
        is Screen.Session -> SessionScreen(model, s.session)
        is Screen.Health -> HealthScreen(model, s.node)
    }
}
