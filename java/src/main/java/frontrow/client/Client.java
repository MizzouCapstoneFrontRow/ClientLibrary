package frontrow.client;

public class Client {
    private String displayName;

    /**
    * Prepares library for use. Must be called first.
    */
    public void InitializeLibrary() {
        throw new UnsupportedOperationException("Not yet implemented");
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
    public void RegisterStream(String streamName, String type, int port) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Registers a sensor feature with the library. Must be called after init and before connection.
    */
    public void RegsiterSensor(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Registers an axis feature with the library. Must be called after init and before connection.
    */
    public void RegsterAxis(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Registers a function with the library. Must be called after init and before connection.
    */
    public void RegsterFunction(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
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
    public void LibraryUpdate(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }

    /**
    * Disconnects and cleans up anything inside the library that requires cleanup.
    */
    public void ShutdownLibrary(/* TODO */) {
        throw new UnsupportedOperationException("Not yet implemented");
    }
}
