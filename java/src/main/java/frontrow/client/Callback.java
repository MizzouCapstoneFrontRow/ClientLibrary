package frontrow.client;

@FunctionalInterface
public interface Callback {
    Object[] call(Object[] params);
}
