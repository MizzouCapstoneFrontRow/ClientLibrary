package frontrow.client;

import java.util.HashMap;

public class Client {
    // Fields
    private String displayName;
    private HashMap<String, Function> functions;


    // Methods
    /**
    * Prepares library for use. Must be called first.
    */
    public void InitializeLibrary() {
        this.functions = new HashMap<>();
    }

    /**
    * Sets the display name of the machine. Must be called after init and before connection.
    * @param displayName String that is the name of the machine
    */
    public void SetName(String displayName) {
        this.displayName = displayName;
    }

    /**
    * Registers a stream feature with the library. Must be called after init and before connection.
    * @param streamName      String that is the name of the feature
    * @param type            String that defines what type the stream is
    * @param port            int that defines what port the stream is defined on
    * @param characteristics Array of string pairs that defines the possible data this stream provides
    */
    public void RegisterStream(String streamName, String type, int port/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Registers a sensor feature with the library. Must be called after init and before connection.
    */
    public void RegisterSensor(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Registers an axis feature with the library. Must be called after init and before connection.
    */
    public void RegisterAxis(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Registers a function with the library. Must be called after init and before connection.
    * @param name       String that is the name of the function
    * @param parameters The paramters of the function.
    * @param returns    The return types of the function.
    * @param callback   The callback.
    */
    public void RegisterFunction(String name, Parameter[] parameters, Parameter[] returns, Callback callback) {
        this.functions.put(name, new Function(parameters, returns, callback));
    }

    /**
    * Connects to server. Library must be initialized and machine description prepared before connecting.
    */
    public void ConnectToServer(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Updates internal library state and calls any necessary callbacks.
    */
    public void LibraryUpdate() {
        // Testing
        System.out.println(functions);
        Callback print = functions.get("print").callback;
        print.call("Hello from a callback!");

        Callback multiply = functions.get("multiply").callback;
        Object[] result = multiply.call(4, 5);
        System.out.println("4 * 5 = " + (Integer)result[0]);

        Callback average = functions.get("average").callback;
        double[] arr = new double[]{1, 2, 3, 4, 5, 20};
        result = average.call(arr);
        System.out.println("avg(" + arr + ") = " + (Double)result[0]);

        Callback sequence = functions.get("sequence").callback;
        result = sequence.call((Integer)20);
        int[] seq = (int[])result[0];
        System.out.print("seq(" + 20 + ") = {");
        for (int i : seq) {
            System.out.print(i + ", ");
        }
        System.out.println("}");
    }

    /**
    * Disconnects and cleans up anything inside the library that requires cleanup.
    */
    public void ShutdownLibrary() {
    }
}
