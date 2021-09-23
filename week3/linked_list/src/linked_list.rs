use std::fmt;
use std::option::Option;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

pub struct LinkedListIter<'a, T> {
    current: &'a Option<Box<Node<T>>>
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<'a, T> Iterator for LinkedListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        match self.current {
            Some(node) => {
                self.current = &node.next;
                Some(&node.value)
            },
            None => None
        }
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node { value: self.value.clone(), next: self.next.clone() }
    }
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Node<T>) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {value: value, next: next}
    }
}

impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        LinkedList { head: self.head.clone(), size: self.size}
    }
}

impl<T: PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, rhs: &LinkedList<T>) -> bool {
        if self.size != rhs.size {
            return false;
        } 
        let mut l: &Option<Box<Node<T>>> = &self.head;
        let mut r: &Option<Box<Node<T>>> = &rhs.head;
        loop {
            match l {
                Some(node) => {
                    let rnode = r.as_ref().unwrap();
                    if *node != *rnode {
                        break false;
                    }
                    l = &node.next;
                    r = &rnode.next;
                },
                None => break true,
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a LinkedList<T> {

    type Item = &'a T;
    type IntoIter = LinkedListIter<'a, T>;

    fn into_iter(self) -> LinkedListIter<'a, T> {
        LinkedListIter { current: &self.head }
    }
}

impl<T: Clone> Iterator for LinkedList<T> {

    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.pop_front()
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {head: None, size: 0}
    }
    
    pub fn get_size(&self) -> usize {
        self.size
    }
    
    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }
    
    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::<T>::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}


impl<T: fmt::Display > fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                },
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}



