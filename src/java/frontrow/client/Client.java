package frontrow.client;

public class Client {
    static {
        System.loadLibrary("TEST");
    }
    public static void test() {
        System.out.println("Printing from Java!");
        native_method();
    }
    public static native void native_method();
}
