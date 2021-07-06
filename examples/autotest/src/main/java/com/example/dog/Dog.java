package com.example.dog;

import com.example.animal.Animal;

public class Dog implements Animal {
    public String name = "Furry";
    public String feels_like() {
        return name;
    }
}