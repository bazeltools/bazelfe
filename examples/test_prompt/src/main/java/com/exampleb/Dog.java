package com.exampleb;

import com.animal.Animal;

public class Dog implements Animal{
    public String name = "Furry";
    public String feels_like() {
        return name;
    }
}