package frontrow.client;

public class NativeCallback implements Callback {
    static {
        System.loadLibrary("NativeCallback");
    }

    private Parameter[] parameters;
    private Parameter[] returns;
    private long function_pointer;

    /**
    * Only called from JNI.
    */
    private NativeCallback(Parameter[] parameters, Parameter[] returns, long function_pointer) {
        this.parameters = parameters;
        this.returns = returns;
        this.function_pointer = function_pointer;
    }

    /**
    * Implement Callback interface. Native so it can call C function_pointer.
    */
    public native Object[] call(Object[] params);
}
