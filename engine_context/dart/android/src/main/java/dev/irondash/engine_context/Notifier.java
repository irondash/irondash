package dev.irondash.engine_context;

import androidx.annotation.Keep;

@Keep
public interface Notifier {
    void onNotify(Object value);
    void destroy();
}

@Keep
class NativeNotifier implements Notifier {
    long mNativeData;

    NativeNotifier(long mNativeData) {
        this.mNativeData = mNativeData;
    }

    public native void onNotify(Object value);

    // Must be called before garbage collected
    public native void destroy();
}
