package hev.sockstun;

public class TProxyService {
    static {
        try {
            System.loadLibrary("hev-socks5-tunnel");
        } catch (Throwable t) {
            t.printStackTrace();
        }
    }

    public static native void TProxyStartService(String configPath, int fd);
    public static native void TProxyStopService();
    public static native long[] TProxyGetStats();
}
