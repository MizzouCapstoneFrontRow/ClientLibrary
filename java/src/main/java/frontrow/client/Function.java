package frontrow.client;

class Function {
    Parameter[] parameters;
    Parameter[] returns;
    Callback callback;
    Function(Parameter[] parameters, Parameter[] returns, Callback callback) {
        this.parameters = parameters;
        this.returns = returns;
        this.callback = callback;
    }
}
