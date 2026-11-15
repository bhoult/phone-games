package co.hoult.rufuscrush;

import android.app.Activity;
import android.graphics.Rect;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Toast;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;

/**
 * Shows the web build of the game, served straight out of the APK's assets.
 *
 * The page is loaded from a made-up https address so the browser engine
 * treats it as a normal secure site (which WebAssembly streaming and
 * localStorage want). Every request to that address is answered from
 * assets/game; requests to anywhere else are refused. Nothing is fetched from
 * the network, and the app does not hold the INTERNET permission.
 */
public class MainActivity extends Activity {
    private static final String HOST = "rufus-crush.invalid";
    private static final String HOME = "https://" + HOST + "/index.html";

    private WebView web;
    private long lastBack;

    @Override
    protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        web = new WebView(this);
        web.setBackgroundColor(0xFF2B6CB0);
        web.setOverScrollMode(View.OVER_SCROLL_NEVER);
        web.setVerticalScrollBarEnabled(false);
        web.setHorizontalScrollBarEnabled(false);
        web.setLongClickable(false);
        web.setHapticFeedbackEnabled(false);
        web.setOnLongClickListener(v -> true);

        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true); // saved progress lives in localStorage
        s.setMediaPlaybackRequiresUserGesture(false);
        s.setAllowFileAccess(false);
        s.setAllowContentAccess(false);
        s.setSupportZoom(false);
        s.setTextZoom(100);

        web.setWebViewClient(new WebViewClient() {
            @Override
            public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest request) {
                return serve(request.getUrl());
            }

            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                // never navigate away from the game
                return !HOST.equals(request.getUrl().getHost());
            }
        });

        setContentView(web);
        goImmersive();
        keepEdgeSwipesForTheGame();

        if (Build.VERSION.SDK_INT >= 33) {
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                    android.window.OnBackInvokedDispatcher.PRIORITY_DEFAULT, this::backPressed);
        }

        web.loadUrl(HOME);
    }

    /** Answers a request from the files packed into the APK. */
    private WebResourceResponse serve(Uri url) {
        if (!HOST.equals(url.getHost())) {
            return respond(403, "Forbidden", "text/plain", null);
        }
        String path = url.getPath();
        if (path == null || path.isEmpty() || path.equals("/")) {
            path = "/index.html";
        }
        if (path.contains("..")) {
            return respond(403, "Forbidden", "text/plain", null);
        }
        try {
            return respond(200, "OK", mime(path), getAssets().open("game" + path));
        } catch (IOException missing) {
            return respond(404, "Not Found", "text/plain", null);
        }
    }

    private static WebResourceResponse respond(int status, String reason, String mime, java.io.InputStream body) {
        boolean text = mime.startsWith("text/") || mime.endsWith("json");
        Map<String, String> headers = new HashMap<>();
        headers.put("Cache-Control", "no-store");
        return new WebResourceResponse(mime, text ? "utf-8" : null, status, reason, headers,
                body != null ? body : new ByteArrayInputStream(new byte[0]));
    }

    private static String mime(String path) {
        if (path.endsWith(".html")) return "text/html";
        if (path.endsWith(".js")) return "text/javascript";
        if (path.endsWith(".wasm")) return "application/wasm";
        if (path.endsWith(".png")) return "image/png";
        if (path.endsWith(".webmanifest")) return "application/manifest+json";
        return "application/octet-stream";
    }

    /** Full screen, drawn under the status bar and camera cutout; bars return with a swipe. */
    private void goImmersive() {
        getWindow().setDecorFitsSystemWindows(false);
        WindowInsetsController c = getWindow().getInsetsController();
        if (c != null) {
            c.hide(WindowInsets.Type.systemBars());
            c.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
        }
    }

    /**
     * With gesture navigation, a swipe that starts at the screen edge means
     * "back". The board's outer columns sit right at the edge, so ask the
     * system to leave the middle of each edge alone (it allows 200dp a side).
     */
    private void keepEdgeSwipesForTheGame() {
        web.addOnLayoutChangeListener((v, l, t, r, b, ol, ot, or, ob) -> {
            float dp = getResources().getDisplayMetrics().density;
            int w = r - l, h = b - t;
            int strip = (int) (40 * dp), tall = (int) (200 * dp);
            // centred a little below the middle, where the board is
            int top = Math.max(0, Math.min(h - tall, (int) (h * 0.58f) - tall / 2));
            v.setSystemGestureExclusionRects(Arrays.asList(
                    new Rect(0, top, strip, top + tall),
                    new Rect(w - strip, top, w, top + tall)));
        });
    }

    /** One back press is easy to do by accident mid-swipe; leaving takes two. */
    private void backPressed() {
        long now = System.currentTimeMillis();
        if (now - lastBack < 2000) {
            finish();
        } else {
            lastBack = now;
            Toast.makeText(this, "Back again to leave Rufus", Toast.LENGTH_SHORT).show();
        }
    }

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        backPressed();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            goImmersive();
        }
    }

    @Override
    protected void onPause() {
        // the page sees itself become hidden and silences its audio
        web.onPause();
        web.pauseTimers();
        super.onPause();
    }

    @Override
    protected void onResume() {
        super.onResume();
        web.resumeTimers();
        web.onResume();
    }

    @Override
    protected void onDestroy() {
        web.destroy();
        super.onDestroy();
    }
}
