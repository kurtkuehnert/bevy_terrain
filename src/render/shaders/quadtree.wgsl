


fn evaluate_layer() { // for each node
    calculate invalidated

    if invalidated {
        request deactivation // nothing to do for children

        lookup parent and copy its index
    }
    else {
        if in active range {
            if not active {
                request activation
            }
        }
        else {
            if active {
                request deactivation
            }
        }
    }
}