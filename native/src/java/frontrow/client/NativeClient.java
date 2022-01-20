package frontrow.client;

public class NativeClient {
    static {
        System.loadLibrary("NativeClient");
    }
    public static void test() {
        System.out.println("Printing from Java!");
        native_method();
    }
    public static native void native_method();
}
